use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce as AesNonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use p256::PublicKey;
use p256::SecretKey;
use p256::ecdh::diffie_hellman;
use p256::elliptic_curve::Generate;
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use rand::Rng;
use sha2::Sha256;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use url::Url;

const PROTOCOL_VERSION: u8 = 2;
const PROTOCOL_INFO: &[u8] = b"modrinth-app-auth-v2";
const MAX_MESSAGE_LEN: usize = 4096;
const MAX_HEADER_LEN: usize = 8192;
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);

/// Pending external-browser Modrinth sign-in. The session token stays sealed
/// until it is decrypted inside this backend.
pub struct ModrinthLoginSession {
    pub auth_request_uri: String,
    seal: AuthSeal,
    listener: Option<JoinHandle<Result<Option<String>, String>>>,
}

impl ModrinthLoginSession {
    pub async fn begin(begin_url: &str) -> Result<Self, String> {
        let mut auth_url =
            Url::parse(begin_url).map_err(|error| error.to_string())?;
        validate_modrinth_auth_url(&auth_url)?;

        let listener = theseus::prelude::tcp_listen_any_loopback()
            .await
            .map_err(|error| error.to_string())?;
        let socket =
            listener.local_addr().map_err(|error| error.to_string())?;
        let seal = AuthSeal::generate();
        let public_key = seal.public_key()?;
        {
            let mut query = auth_url.query_pairs_mut();
            query.append_pair("launcher", "true");
            query.append_pair("protocol", "2");
            query.append_pair("key", &public_key);
            query.append_pair("nonce", &seal.nonce);
            query
                .append_pair("ipver", if socket.is_ipv4() { "4" } else { "6" });
            query.append_pair("port", &socket.port().to_string());
            query.append_pair("add_account", "true");
        }
        let auth_request_uri = auth_url.into();
        let nonce = seal.nonce.clone();
        let listener = tokio::spawn(async move {
            listen_for_callback(listener, socket, nonce).await
        });

        Ok(Self {
            auth_request_uri,
            seal,
            listener: Some(listener),
        })
    }

    pub async fn finish(mut self) -> Result<String, String> {
        let result = {
            let listener = self.listener.as_mut().ok_or_else(|| {
                "Modrinth sign-in listener is unavailable".to_string()
            })?;
            listener.await.map_err(|error| {
                format!("Modrinth sign-in listener stopped: {error}")
            })??
        };
        self.listener.take();
        let sealed = result.ok_or_else(|| {
            "Modrinth sign-in was canceled or expired".to_string()
        })?;
        self.seal.open(&sealed)
    }
}

impl Drop for ModrinthLoginSession {
    fn drop(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }
}

struct AuthSeal {
    secret: SecretKey,
    nonce: String,
}

impl AuthSeal {
    fn generate() -> Self {
        let mut rng = rand::rng();
        let secret = SecretKey::generate_from_rng(&mut rng);
        let mut nonce_bytes = [0u8; 16];
        rng.fill_bytes(&mut nonce_bytes);
        Self {
            secret,
            nonce: URL_SAFE_NO_PAD.encode(nonce_bytes),
        }
    }

    fn public_key(&self) -> Result<String, String> {
        let der = self
            .secret
            .public_key()
            .to_public_key_der()
            .map_err(|error| error.to_string())?;
        Ok(URL_SAFE_NO_PAD.encode(der.as_bytes()))
    }

    fn open(&self, sealed: &str) -> Result<String, String> {
        if sealed.len() > MAX_MESSAGE_LEN {
            return Err("The sealed sign-in response is too long".into());
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(sealed)
            .map_err(|_| "The sealed sign-in response is invalid")?;
        if bytes.first() != Some(&PROTOCOL_VERSION) || bytes.len() < 3 {
            return Err(
                "The sealed sign-in response has an unsupported format".into(),
            );
        }
        let spki_len = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
        let header_len = 3 + spki_len + 12;
        if spki_len == 0 || spki_len > 256 || bytes.len() < header_len + 16 {
            return Err("The sealed sign-in response is incomplete".into());
        }
        let ephemeral = PublicKey::from_public_key_der(&bytes[3..3 + spki_len])
            .map_err(|_| "The sealed sign-in key is invalid")?;
        let shared = diffie_hellman(
            self.secret.to_nonzero_scalar(),
            ephemeral.as_affine(),
        );
        let mut key = [0u8; 32];
        Hkdf::<Sha256>::new(None, shared.raw_secret_bytes())
            .expand(PROTOCOL_INFO, &mut key)
            .map_err(|_| "The sign-in encryption key is invalid")?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| "The sign-in encryption key is invalid")?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[3 + spki_len..header_len]);
        let plaintext = cipher
            .decrypt(&AesNonce::from(nonce), &bytes[header_len..])
            .map_err(
                |_| "The sealed sign-in response could not be authenticated",
            )?;
        let plaintext = String::from_utf8(plaintext)
            .map_err(|_| "The sign-in response is not valid text")?;
        let (nonce, token) = plaintext
            .split_once('.')
            .ok_or_else(|| "The sign-in response is incomplete".to_string())?;
        if nonce != self.nonce || !is_session_token(token) {
            return Err("The sign-in response failed validation".into());
        }
        Ok(token.to_owned())
    }
}

fn validate_modrinth_auth_url(url: &Url) -> Result<(), String> {
    let host = url.host_str().unwrap_or_default();
    let loopback_http = url.scheme() == "http"
        && (host == "localhost" || host == "127.0.0.1" || host == "[::1]");
    if (!loopback_http && url.scheme() != "https")
        || url.username() != ""
        || url.password().is_some()
    {
        return Err("Modrinth sign-in URL must use HTTPS (or a loopback development URL)".into());
    }
    Ok(())
}

async fn listen_for_callback(
    listener: TcpListener,
    socket: SocketAddr,
    expected_nonce: String,
) -> Result<Option<String>, String> {
    let deadline = tokio::time::sleep(LOGIN_TIMEOUT);
    tokio::pin!(deadline);
    loop {
        let (stream, peer) = tokio::select! {
            _ = &mut deadline => return Ok(None),
            result = listener.accept() => result.map_err(|error| error.to_string())?,
        };
        if !peer.ip().is_loopback() {
            continue;
        }
        if let Some(sealed) =
            handle_callback(stream, socket, &expected_nonce).await?
        {
            return Ok(Some(sealed));
        }
    }
}

async fn handle_callback(
    mut stream: TcpStream,
    socket: SocketAddr,
    expected_nonce: &str,
) -> Result<Option<String>, String> {
    let mut request = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    loop {
        let read = tokio::time::timeout(
            Duration::from_secs(5),
            stream.read(&mut chunk),
        )
        .await
        .map_err(|_| "Sign-in callback request timed out".to_string())?
        .map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(None);
        }
        request.extend_from_slice(&chunk[..read]);
        if request.len() > MAX_HEADER_LEN {
            write_response(&mut stream, 431, "Request header too large")
                .await?;
            return Ok(None);
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "Sign-in callback request is incomplete".to_string())?;
    let request_text =
        std::str::from_utf8(&request[..header_end]).map_err(|_| {
            "Sign-in callback request is not valid HTTP".to_string()
        })?;
    let Some(request_line) = request_text.lines().next() else {
        write_response(&mut stream, 400, "Invalid sign-in callback").await?;
        return Ok(None);
    };
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("GET") {
        write_response(&mut stream, 405, "Method not allowed").await?;
        return Ok(None);
    }
    let Some(target) = parts.next() else {
        write_response(&mut stream, 400, "Invalid sign-in callback").await?;
        return Ok(None);
    };
    let url = Url::parse(&format!("http://{}{}", socket, target))
        .map_err(|_| "Sign-in callback URL is invalid".to_string())?;
    let mut sealed = None;
    let mut nonce = None;
    for (key, value) in url.query_pairs() {
        if value.len() > MAX_MESSAGE_LEN {
            write_response(&mut stream, 400, "Sign-in callback is too long")
                .await?;
            return Ok(None);
        }
        match key.as_ref() {
            "code" => sealed = Some(value.into_owned()),
            "nonce" => nonce = Some(value.into_owned()),
            _ => {}
        }
    }
    let sealed = match (sealed, nonce) {
        (Some(sealed), Some(nonce))
            if nonce == expected_nonce && !sealed.is_empty() =>
        {
            sealed
        }
        _ => {
            write_response(
                &mut stream,
                400,
                "Authorization response not found",
            )
            .await?;
            return Ok(None);
        }
    };
    write_response(
        &mut stream,
        200,
        "Sign-in complete. You can close this page.",
    )
    .await?;
    Ok(Some(sealed))
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    message: &str,
) -> Result<(), String> {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>Northstar sign-in</title><p>{message}</p>"
    );
    let reason = if status == 200 { "OK" } else { "Request Error" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| error.to_string())
}

fn is_session_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix("mra_") else {
        return false;
    };
    (32..=128).contains(&rest.len())
        && rest
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{
        handle_callback, is_session_token, validate_modrinth_auth_url,
    };
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use url::Url;

    #[test]
    fn accepts_only_valid_modrinth_session_token_shapes() {
        assert!(is_session_token(&format!("mra_{}", "a".repeat(32))));
        assert!(!is_session_token("mra_short"));
        assert!(!is_session_token(&format!("mra_{}!", "a".repeat(32))));
    }

    #[test]
    fn validates_external_and_loopback_auth_urls() {
        assert!(
            validate_modrinth_auth_url(
                &Url::parse("https://modrinth.com/auth/sign-in").unwrap()
            )
            .is_ok()
        );
        assert!(
            validate_modrinth_auth_url(
                &Url::parse("http://localhost:3000/auth/sign-in").unwrap()
            )
            .is_ok()
        );
        assert!(
            validate_modrinth_auth_url(
                &Url::parse("http://example.com/auth/sign-in").unwrap()
            )
            .is_err()
        );
        assert!(
            validate_modrinth_auth_url(
                &Url::parse("javascript:alert(1)").unwrap()
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn accepts_matching_loopback_callback_and_ignores_body_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let socket = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(socket).await.unwrap();
        client
            .write_all(b"GET /?code=sealed-token&nonce=fresh HTTP/1.1\r\nHost: localhost\r\n\r\n\xff")
            .await
            .unwrap();
        let (server, peer) = listener.accept().await.unwrap();
        assert!(peer.ip().is_loopback());
        let sealed = handle_callback(server, socket, "fresh").await.unwrap();
        assert_eq!(sealed.as_deref(), Some("sealed-token"));
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200 OK"));
    }

    #[tokio::test]
    async fn rejects_a_loopback_callback_with_a_stale_nonce() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let socket: SocketAddr = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(socket).await.unwrap();
        client
            .write_all(b"GET /?code=sealed-token&nonce=stale HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        assert!(
            handle_callback(server, socket, "fresh")
                .await
                .unwrap()
                .is_none()
        );
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 400"));
    }
}

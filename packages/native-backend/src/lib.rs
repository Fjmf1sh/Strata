//! Asynchronous native frontend access to the Theseus launcher backend.
#![recursion_limit = "256"]

use std::fmt;
use std::sync::OnceLock;
use theseus::{AppEvent, EventState, State, worlds};
use tokio::sync::{broadcast, mpsc, oneshot, watch};

mod migration;
mod modrinth_auth;
pub use migration::{
    ExistingModrinthData, ExistingModrinthMigration,
    copy_existing_modrinth_data, inspect_existing_modrinth_data,
    remember_fresh_start_choice,
};
pub use modrinth_auth::ModrinthLoginSession;

static APP_IDENTIFIER: OnceLock<String> = OnceLock::new();
static SETTINGS_UPDATE_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());
static NATIVE_UI_PREFERENCES_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());
const DEFAULT_QUICK_INSTANCE_COUNT: usize = 5;
const MAX_QUICK_INSTANCE_COUNT: usize = 5;
const MAX_NATIVE_COLOR_THEME: u8 = 8;

fn clamp_native_color_theme(theme: u8) -> u8 {
    theme.min(MAX_NATIVE_COLOR_THEME)
}

fn preferred_dark_theme_from_preferences(
    preferences: &serde_json::Value,
) -> theseus::data::Theme {
    match preferences
        .get("preferred_dark_theme")
        .and_then(serde_json::Value::as_str)
    {
        Some("oled") => theseus::data::Theme::Oled,
        _ => theseus::data::Theme::Dark,
    }
}

fn preferred_dark_theme_name(theme: theseus::data::Theme) -> &'static str {
    match theme {
        theseus::data::Theme::Oled => "oled",
        _ => "dark",
    }
}

fn native_theme_sync_enabled(preferences: &serde_json::Value) -> bool {
    preferences
        .get("sync_theme_across_devices")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn mclogs_client() -> Result<reqwest::Client, BackendError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not initialize mclo.gs request: {error}"
            )))
        })
}

fn mclogs_share_url(
    response: &serde_json::Value,
) -> Result<String, BackendError> {
    if response.get("success").and_then(|value| value.as_bool()) != Some(true) {
        let message = response
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("mclo.gs could not create a share link");
        return Err(BackendError::Operation(theseus::Error::msg(
            message.to_owned(),
        )));
    }
    let share_url = response
        .get("url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            BackendError::Operation(theseus::Error::msg(
                "mclo.gs returned no share URL",
            ))
        })?;
    let parsed = url::Url::parse(share_url).map_err(|error| {
        BackendError::Operation(theseus::Error::msg(format!(
            "mclo.gs returned an invalid share URL: {error}"
        )))
    })?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("mclo.gs") {
        return Err(BackendError::Operation(theseus::Error::msg(
            "mclo.gs returned an untrusted share URL",
        )));
    }
    Ok(parsed.to_string())
}

fn project_follow_url(project_id: &str) -> Result<url::Url, BackendError> {
    let mut url =
        url::Url::parse("https://api.modrinth.com/v3/").map_err(|error| {
            BackendError::Configuration(format!(
                "invalid Modrinth API URL: {error}"
            ))
        })?;
    url.path_segments_mut()
        .map_err(|_| {
            BackendError::Configuration(
                "could not construct Modrinth API URL".into(),
            )
        })?
        .pop_if_empty()
        .extend(["project", project_id, "follow"]);
    Ok(url)
}

fn collection_url(collection_id: &str) -> Result<url::Url, BackendError> {
    let mut url =
        url::Url::parse("https://api.modrinth.com/v3/").map_err(|error| {
            BackendError::Configuration(format!(
                "invalid Modrinth API URL: {error}"
            ))
        })?;
    url.path_segments_mut()
        .map_err(|_| {
            BackendError::Configuration(
                "could not construct Modrinth API URL".into(),
            )
        })?
        .pop_if_empty()
        .extend(["collection", collection_id]);
    Ok(url)
}

fn hosting_backup_url(
    server_id: &str,
    world_id: &str,
    tail: &[&str],
) -> Result<url::Url, BackendError> {
    let mut url = url::Url::parse("https://archon.modrinth.com/v1/").map_err(
        |error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not build Hosting backup URL: {error}"
            )))
        },
    )?;
    let mut segments = url.path_segments_mut().map_err(|_| {
        BackendError::Operation(theseus::Error::msg(
            "could not build Hosting backup URL",
        ))
    })?;
    segments
        .pop_if_empty()
        .push("servers")
        .push(server_id)
        .push("worlds")
        .push(world_id);
    for part in tail {
        segments.push(part);
    }
    drop(segments);
    Ok(url)
}

fn hosting_server_url(server_id: &str) -> Result<url::Url, BackendError> {
    let mut url = url::Url::parse("https://archon.modrinth.com/v1/").map_err(
        |error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not build Hosting server URL: {error}"
            )))
        },
    )?;
    url.path_segments_mut()
        .map_err(|_| {
            BackendError::Operation(theseus::Error::msg(
                "could not build Hosting server URL",
            ))
        })?
        .pop_if_empty()
        .extend(["servers", server_id]);
    Ok(url)
}

#[derive(Clone)]
pub enum NativeEvent {
    Event(AppEvent),
    Lagged(u64),
}

async fn forward_events(
    mut source: broadcast::Receiver<AppEvent>,
) -> mpsc::UnboundedReceiver<NativeEvent> {
    let (event_sender, receiver) = mpsc::unbounded_channel();
    let (forwarder_started, wait_for_forwarder) = oneshot::channel();
    tokio::spawn(async move {
        let _ = forwarder_started.send(());
        loop {
            match source.recv().await {
                Ok(event) => {
                    if event_sender.send(NativeEvent::Event(event)).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    if event_sender.send(NativeEvent::Lagged(count)).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    let _ = wait_for_forwarder.await;
    receiver
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupStatus {
    Starting,
    Ready,
    Failed(String),
}

#[derive(Debug)]
pub enum BackendError {
    Startup(String),
    Authentication(String),
    Configuration(String),
    Operation(theseus::Error),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Startup(message) => {
                write!(f, "backend initialization failed: {message}")
            }
            Self::Authentication(message) => write!(f, "{message}"),
            Self::Configuration(message) => write!(f, "{message}"),
            Self::Operation(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Owns a readiness subscription. Clone it for independent UI tasks.
#[derive(Clone)]
pub struct NativeBackend {
    startup: watch::Receiver<StartupStatus>,
}

pub struct CreateInstance {
    pub name: String,
    pub game_version: String,
    pub loader: theseus::data::ModLoader,
    pub loader_version: Option<String>,
    pub icon_path: Option<String>,
    pub icon_config: Option<theseus::data::IconConfig>,
    pub link: theseus::data::InstanceLink,
}

/// Account details suitable for display without exposing authentication tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinecraftAccount {
    pub id: uuid::Uuid,
    pub username: String,
    pub active: bool,
}

/// A Microsoft sign-in session without exposing credential tokens to the UI.
pub struct MinecraftLoginSession {
    pub auth_request_uri: String,
    flow: theseus::data::MinecraftLoginFlow,
    started_at: std::time::Instant,
}

/// A content-service account without its session token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentServiceAccount {
    pub user_id: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ContentServiceCollection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub projects: Vec<String>,
}

/// Non-secret server fields used to render the native Hosting list.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServer {
    pub server_id: String,
    pub name: String,
    pub owner_id: String,
    pub status: String,
    pub game: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub mc_version: Option<String>,
    pub net: HostingServerNetwork,
    #[serde(default)]
    pub is_medal: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerNetwork {
    pub domain: String,
    pub port: u16,
}

/// Non-secret fields for a native Hosting server overview.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerDetails {
    #[serde(flatten)]
    pub server: HostingServer,
    #[serde(default)]
    pub current_user_permissions: serde_json::Value,
    #[serde(default)]
    pub backup_quota: u64,
    #[serde(default)]
    pub used_backup_quota: u64,
    pub suspension_reason: Option<String>,
    pub datacenter: Option<String>,
    pub locked_since: Option<String>,
    #[serde(default)]
    pub notices: Vec<HostingServerNotice>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackupUserInfo {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackupOperation {
    #[serde(default)]
    pub operation_type: String,
    #[serde(default)]
    pub user_info: Option<HostingBackupUserInfo>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackup {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub status: String,
    #[serde(default)]
    pub automated: bool,
    #[serde(default)]
    pub history: Vec<HostingBackupOperation>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackupOptions {
    #[serde(default)]
    pub split_automatic: usize,
    #[serde(default)]
    pub split_manual: usize,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub interval: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackups {
    #[serde(skip)]
    pub world_id: String,
    #[serde(default)]
    pub backups: Vec<HostingBackup>,
    #[serde(default)]
    pub deleted_backups: Vec<HostingDeletedBackup>,
    #[serde(default)]
    pub manual_count: usize,
    #[serde(default)]
    pub auto_count: usize,
    pub next_auto_backup_at: Option<String>,
    pub options: HostingBackupOptions,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct HostingBackupOptionsPatch {
    pub split_automatic: usize,
    pub split_manual: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingBackupOptionsUpdate {
    pub applied: bool,
    #[serde(default)]
    pub backup_ids: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingDeletedBackup {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub status: String,
    #[serde(default)]
    pub automated: bool,
    #[serde(default)]
    pub created_by: Option<HostingBackupUserInfo>,
    pub scheduled_delete: String,
    pub undelete_deadline: String,
    #[serde(default)]
    pub deleted_suspension: bool,
    #[serde(default)]
    pub can_undelete: bool,
    #[serde(default)]
    pub permanently_deleted: bool,
}

#[derive(serde::Deserialize)]
struct HostingServerWorldsResponse {
    #[serde(default)]
    worlds: Vec<HostingServerWorld>,
}

#[derive(serde::Deserialize)]
struct HostingServerWorld {
    id: String,
    #[serde(default)]
    is_active: bool,
}

#[derive(serde::Deserialize)]
struct HostingNodeResponse {
    node: Option<HostingNode>,
}

#[derive(serde::Deserialize)]
struct HostingNode {
    instance: String,
}

#[derive(serde::Deserialize)]
struct HostingBackupDownloadAuthorization {
    token: String,
}

fn hosting_node_backup_url(
    node_instance: &str,
    world_id: &str,
    backup_id: &str,
    authorize: bool,
    token: Option<&str>,
) -> Result<url::Url, BackendError> {
    let base = node_instance
        .trim_end_matches('/')
        .strip_suffix("/modrinth/v0/fs")
        .or_else(|| {
            node_instance
                .trim_end_matches('/')
                .rsplit_once("/modrinth/v")
                .and_then(|(base, version)| {
                    version.strip_suffix("/fs").map(|_| base)
                })
        })
        .unwrap_or(node_instance.trim_end_matches('/'));
    let base = if base.starts_with("https://") || base.starts_with("http://") {
        base.to_owned()
    } else {
        format!("https://{base}")
    };
    let mut url = url::Url::parse(&base).map_err(|error| {
        BackendError::Configuration(format!(
            "invalid Hosting node URL: {error}"
        ))
    })?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(BackendError::Configuration(
            "Hosting node URL must be a credential-free HTTPS URL".into(),
        ));
    }
    url.set_path("");
    url.path_segments_mut()
        .map_err(|_| {
            BackendError::Configuration("invalid Hosting node URL".into())
        })?
        .pop_if_empty()
        .extend(["v1", "worlds", world_id, "backups", backup_id, "download"]);
    if authorize {
        url.path_segments_mut()
            .map_err(|_| {
                BackendError::Configuration("invalid Hosting node URL".into())
            })?
            .push("authorize");
        url.query_pairs_mut().append_pair("format", "zip");
    } else if let Some(token) = token {
        url.query_pairs_mut().append_pair("token", token);
    }
    Ok(url)
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerNotice {
    pub id: u64,
    pub dismissable: bool,
    pub title: String,
    pub message: String,
    pub level: String,
    pub announced: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerAccessUser {
    pub user: HostingServerAccessProfile,
    pub added_on: Option<String>,
    pub last_invite_sent: Option<String>,
    #[serde(default)]
    pub permissions: serde_json::Value,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerReinviteResponse {
    pub sent: bool,
    pub cooldown_seconds: Option<u64>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HostingServerAccessProfile {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum HostingServerAccessRole {
    Owner,
    Viewer,
    Editor,
}

impl HostingServerAccessRole {
    fn as_api_value(self) -> &'static str {
        match self {
            Self::Owner => "Owner",
            Self::Viewer => "Viewer",
            Self::Editor => "Editor",
        }
    }
}

impl HostingServerAccessUser {
    pub fn role(&self) -> HostingServerAccessRole {
        const OWNER_MASK: u64 = u64::MAX ^ ((1 << 15) - 1);
        const EDITOR_MASK: u64 = (1 << 61)
            | (1 << 60)
            | (1 << 59)
            | (1 << 58)
            | (1 << 57)
            | (1 << 56);
        let permissions = if let Some(value) = self.permissions.as_u64() {
            Some(value)
        } else if let Some(value) = self.permissions.as_i64() {
            Some(value as u64)
        } else {
            self.permissions.as_str().and_then(|value| {
                if let Ok(mask) = value.parse::<u64>() {
                    Some(mask)
                } else {
                    let scopes = value.split('|').map(str::trim);
                    let mut mask = 0u64;
                    for scope in scopes {
                        mask |= match scope {
                            "SERVER_ADMIN" => OWNER_MASK,
                            "EXEC_COMMANDS" => 1 << 61,
                            "FILES_WRITE" => 1 << 60,
                            "SETUP" => 1 << 59,
                            "BACKUPS" => 1 << 58,
                            "ADVANCED" => 1 << 57,
                            "RESET_SERVER" => 1 << 56,
                            _ => 0,
                        };
                    }
                    Some(mask)
                }
            })
        };
        if permissions.is_some_and(|mask| mask & OWNER_MASK == OWNER_MASK) {
            HostingServerAccessRole::Owner
        } else if permissions.is_some_and(|mask| mask & EDITOR_MASK != 0) {
            HostingServerAccessRole::Editor
        } else {
            HostingServerAccessRole::Viewer
        }
    }
}

#[derive(serde::Deserialize)]
struct HostingWebSocketAuth {
    url: String,
    token: String,
}

fn hosting_websocket_url(url: &str) -> String {
    if url.starts_with("wss://") || url.starts_with("ws://") {
        url.to_owned()
    } else if let Some(url) = url.strip_prefix("https://") {
        format!("wss://{url}")
    } else if let Some(url) = url.strip_prefix("http://") {
        format!("ws://{url}")
    } else {
        format!("wss://{url}")
    }
}

fn parse_hosting_server_event(text: &str) -> Option<HostingServerEvent> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let event = value.get("event")?.as_str()?;
    match event {
        "log" => Some(HostingServerEvent::Log(
            value.get("message")?.as_str()?.to_owned(),
        )),
        "log4j" => {
            let message =
                value.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let throwable = value
                .get("throwable")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Some(HostingServerEvent::Log(if throwable.is_empty() {
                message.to_owned()
            } else if message.is_empty() {
                throwable.to_owned()
            } else {
                format!("{message}\n{throwable}")
            }))
        }
        "power-state" => Some(HostingServerEvent::PowerState {
            state: value.get("state")?.as_str()?.to_owned(),
            oom_killed: value.get("oom_killed").and_then(|v| v.as_bool()),
            exit_code: value
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok()),
        }),
        "state" => Some(HostingServerEvent::State {
            power_variant: value.get("power_variant")?.as_str()?.to_owned(),
            uptime: value.get("uptime").and_then(|v| v.as_u64()).unwrap_or(0),
            was_oom: value.get("was_oom").and_then(|v| v.as_bool()),
            exit_code: value
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok()),
        }),
        "stats" => Some(HostingServerEvent::Stats {
            cpu_percent: value
                .get("cpu_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            ram_usage_bytes: value
                .get("ram_usage_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            ram_total_bytes: value
                .get("ram_total_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            storage_usage_bytes: value
                .get("storage_usage_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            storage_total_bytes: value
                .get("storage_total_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        }),
        "uptime" => {
            Some(HostingServerEvent::Uptime(value.get("uptime")?.as_u64()?))
        }
        "auth-incorrect" => Some(HostingServerEvent::AuthenticationFailed),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HostingPowerAction {
    Start,
    Stop,
    Restart,
    Kill,
}

impl HostingPowerAction {
    fn as_api_value(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Restart => "Restart",
            Self::Kill => "Kill",
        }
    }
}

#[derive(Clone, Debug)]
pub enum HostingServerEvent {
    Log(String),
    PowerState {
        state: String,
        oom_killed: Option<bool>,
        exit_code: Option<i32>,
    },
    State {
        power_variant: String,
        uptime: u64,
        was_oom: Option<bool>,
        exit_code: Option<i32>,
    },
    Stats {
        cpu_percent: f64,
        ram_usage_bytes: u64,
        ram_total_bytes: u64,
        storage_usage_bytes: u64,
        storage_total_bytes: u64,
    },
    Uptime(u64),
    AuthenticationFailed,
    Disconnected,
}

pub struct HostingServerConnection {
    commands: tokio::sync::mpsc::UnboundedSender<String>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl HostingServerConnection {
    pub fn send_command(&self, command: String) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("Enter a console command first.".into());
        }
        if command.len() > 4096 {
            return Err(
                "Console commands must be 4096 characters or fewer.".into()
            );
        }
        self.commands
            .send(command)
            .map_err(|_| "The server console is disconnected.".into())
    }
}

impl Drop for HostingServerConnection {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[derive(serde::Deserialize)]
struct HostingServerListResponse {
    servers: Vec<HostingServer>,
}

/// Public Modrinth news content used by the native app sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewsArticle {
    pub title: String,
    pub thumbnail: String,
    pub summary: String,
    pub date: String,
    pub link: String,
}

/// A Modrinth friend and live presence summary for the native sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeFriend {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub accepted: bool,
    pub incoming_request: bool,
    pub online: bool,
    pub playing_on: Option<String>,
    pub last_updated: Option<i64>,
    pub created: i64,
}

pub struct ExportPack {
    pub instance_id: String,
    pub destination: std::path::PathBuf,
    pub included_paths: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub version_id: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}

/// A resolved content plan and its running installation.
pub struct ContentInstallation {
    pub plan: theseus::data::ResolveContentPlan,
    pub task: tokio::task::JoinHandle<theseus::Result<()>>,
}

impl NativeBackend {
    /// Initialize event delivery before starting backend initialization.
    ///
    /// Call from a Tokio runtime. The returned event receiver is registered
    /// before initialization begins, so startup events can be observed.
    pub async fn start(
        app_identifier: String,
    ) -> theseus::Result<(Self, mpsc::UnboundedReceiver<NativeEvent>)> {
        if !valid_app_identifier(&app_identifier) {
            return Err(theseus::Error::msg(
                "application identifier must contain only letters, digits, dots, underscores or hyphens, with no empty dot-separated segments",
            ));
        }
        if let Some(state) = State::get_if_initialized()
            && state.directories.app_identifier != app_identifier
        {
            return Err(theseus::Error::msg(
                "Theseus is already initialized for a different application identifier",
            ));
        }
        let configured = APP_IDENTIFIER.get_or_init(|| app_identifier.clone());
        if configured != &app_identifier {
            return Err(theseus::Error::msg(
                "native backend was already started for a different application identifier",
            ));
        }
        theseus::install_crypto_provider();
        let events = EventState::init().await?;
        let receiver = forward_events(events.subscribe()).await;
        let (sender, startup) = watch::channel(StartupStatus::Starting);
        tokio::spawn(async move {
            let status = match State::init(app_identifier).await {
                Ok(()) => StartupStatus::Ready,
                Err(error) => StartupStatus::Failed(format!("{error:#}")),
            };
            sender.send_replace(status);
        });
        Ok((Self { startup }, receiver))
    }

    pub fn startup_status(&self) -> watch::Receiver<StartupStatus> {
        self.startup.clone()
    }

    pub async fn wait_ready(&self) -> Result<(), BackendError> {
        let mut status = self.startup.clone();
        loop {
            match status.borrow_and_update().clone() {
                StartupStatus::Ready => return Ok(()),
                StartupStatus::Failed(message) => {
                    return Err(BackendError::Startup(message));
                }
                StartupStatus::Starting => {}
            }
            if status.changed().await.is_err() {
                return Err(BackendError::Startup(
                    "initialization task ended before reporting a result"
                        .to_string(),
                ));
            }
        }
    }

    /// Return the persisted first-run checklist used by the official app.
    pub async fn onboarding_checklist(
        &self,
    ) -> Result<theseus::data::OnboardingChecklist, BackendError> {
        self.wait_ready().await?;
        theseus::onboarding_checklist::get()
            .await
            .map_err(BackendError::Operation)
    }

    /// Persist the user's choice to skip the first-instance welcome screen.
    pub async fn skip_create_instance_onboarding(
        &self,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::onboarding_checklist::mark_created_instance()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn list_instances(
        &self,
    ) -> Result<Vec<theseus::data::InstanceMetadata>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::list()
            .await
            .map_err(BackendError::Operation)
    }

    /// Return instance IDs hidden from the authenticated user's library.
    pub async fn hidden_instance_ids(
        &self,
    ) -> Result<Vec<String>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_hidden_instance_ids()
            .await
            .map_err(BackendError::Operation)
    }

    /// Hide or restore an instance in the authenticated user's library.
    pub async fn set_instance_hidden(
        &self,
        instance_id: &str,
        hidden: bool,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::set_instance_hidden(instance_id, hidden)
            .await
            .map_err(BackendError::Operation)
    }

    /// Load the most recently played worlds and servers for the home screen.
    pub async fn recent_worlds(
        &self,
        limit: usize,
    ) -> Result<Vec<theseus::worlds::WorldWithInstance>, BackendError> {
        self.wait_ready().await?;
        theseus::worlds::get_recent_worlds(
            limit,
            enumset::EnumSet::only(theseus::worlds::DisplayStatus::Normal),
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn settings(
        &self,
    ) -> Result<theseus::data::Settings, BackendError> {
        self.wait_ready().await?;
        theseus::settings::get()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn global_screenshot_sync(&self) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_global_synced_options()
            .await
            .map(|options| options.screenshots)
            .map_err(BackendError::Operation)
    }

    pub async fn set_global_screenshot_sync(
        &self,
        enabled: bool,
    ) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        theseus::instance::set_global_synced_option(
            theseus::data::SyncedOption::Screenshots,
            enabled,
            None,
        )
        .await
        .map(|options| options.screenshots)
        .map_err(BackendError::Operation)
    }

    pub async fn minecraft_accounts(
        &self,
    ) -> Result<Vec<MinecraftAccount>, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::minecraft_auth::users()
            .await
            .map_err(BackendError::Operation)?;
        Ok(credentials
            .into_iter()
            .map(|account| MinecraftAccount {
                id: account.offline_profile.id,
                username: account.offline_profile.name,
                active: account.active,
            })
            .collect())
    }

    pub async fn begin_minecraft_login(
        &self,
    ) -> Result<MinecraftLoginSession, BackendError> {
        self.wait_ready().await?;
        let flow = theseus::minecraft_auth::begin_login()
            .await
            .map_err(BackendError::Operation)?;
        Ok(MinecraftLoginSession {
            auth_request_uri: flow.auth_request_uri.clone(),
            flow,
            started_at: std::time::Instant::now(),
        })
    }

    pub async fn finish_minecraft_login(
        &self,
        session: MinecraftLoginSession,
        code: &str,
    ) -> Result<MinecraftAccount, BackendError> {
        self.wait_ready().await?;
        if session.started_at.elapsed() > std::time::Duration::from_secs(600) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "Microsoft sign-in expired; start a new sign-in flow",
            )));
        }
        let credentials =
            theseus::minecraft_auth::finish_login(code, session.flow)
                .await
                .map_err(BackendError::Operation)?;
        Ok(MinecraftAccount {
            id: credentials.offline_profile.id,
            username: credentials.offline_profile.name,
            active: credentials.active,
        })
    }

    pub async fn default_minecraft_account(
        &self,
    ) -> Result<Option<uuid::Uuid>, BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_auth::get_default_user()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_default_minecraft_account(
        &self,
        id: uuid::Uuid,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_auth::set_default_user(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn remove_minecraft_account(
        &self,
        id: uuid::Uuid,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_auth::remove_user(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn content_service_accounts(
        &self,
    ) -> Result<Vec<ContentServiceAccount>, BackendError> {
        self.wait_ready().await?;
        let accounts = theseus::mr_auth::get_all()
            .await
            .map_err(BackendError::Operation)?;
        Ok(accounts
            .into_iter()
            .map(|account| ContentServiceAccount {
                user_id: account.user_id,
                active: account.active,
            })
            .collect())
    }

    pub async fn begin_content_service_login(
        &self,
    ) -> Result<ModrinthLoginSession, BackendError> {
        self.wait_ready().await?;
        let begin_url = theseus::mr_auth::authenticate_begin_flow(
            theseus::mr_auth::ModrinthAuthFlow::SignIn,
        )
        .await
        .map_err(BackendError::Operation)?;
        ModrinthLoginSession::begin(begin_url)
            .await
            .map_err(BackendError::Authentication)
    }

    pub async fn finish_content_service_login(
        &self,
        session: ModrinthLoginSession,
    ) -> Result<ContentServiceAccount, BackendError> {
        self.wait_ready().await?;
        let session_token = session
            .finish()
            .await
            .map_err(BackendError::Authentication)?;
        let credentials =
            theseus::mr_auth::authenticate_finish_flow(&session_token)
                .await
                .map_err(BackendError::Operation)?;
        Ok(ContentServiceAccount {
            user_id: credentials.user_id,
            active: credentials.active,
        })
    }

    pub async fn set_active_content_service_account(
        &self,
        user_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::mr_auth::set_active(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn remove_content_service_account(
        &self,
        user_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::mr_auth::remove_user(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn hosting_servers(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<HostingServer>, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to view Hosting servers.".into(),
                )
            })?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not initialize mclo.gs crash insights: {error}"
                )))
            })?;
        let response = client
            .get("https://archon.modrinth.com/modrinth/v0/servers")
            .query(&[
                ("limit", limit.clamp(1, 100).to_string()),
                ("offset", offset.to_string()),
            ])
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting servers: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting servers: {error}"
                )))
            })?
            .json::<HostingServerListResponse>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting server data: {error}"
                )))
            })?;
        Ok(response.servers)
    }

    pub async fn hosting_server_details(
        &self,
        server_id: &str,
    ) -> Result<HostingServerDetails, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to view Hosting servers.".into(),
                )
            })?;
        reqwest::Client::new()
            .get(format!(
                "https://archon.modrinth.com/modrinth/v0/servers/{server_id}"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting server details: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting server details: {error}"
                )))
            })?
            .json::<HostingServerDetails>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting server details: {error}"
                )))
            })
    }

    pub async fn hosting_server_backups(
        &self,
        server_id: &str,
    ) -> Result<HostingBackups, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to view Hosting backups.".into(),
                )
            })?;
        let client = reqwest::Client::new();
        let server = client
            .get(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, &credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting world information: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting world information: {error}"
                )))
            })?
            .json::<HostingServerWorldsResponse>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting world information: {error}"
                )))
            })?;
        let world_id = server
            .worlds
            .iter()
            .find(|world| world.is_active)
            .or_else(|| server.worlds.first())
            .map(|world| world.id.clone())
            .ok_or_else(|| {
                BackendError::Operation(theseus::Error::msg(
                    "this Hosting server does not have a world to back up",
                ))
            })?;
        let mut backups = client
            .get(hosting_backup_url(
                server_id,
                &world_id,
                &["backups-queue", "with-options"],
            )?)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, &credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting backups: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting backups: {error}"
                )))
            })?
            .json::<HostingBackups>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting backups: {error}"
                )))
            })?;
        backups.world_id = world_id;
        backups
            .backups
            .sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(backups)
    }

    pub async fn hosting_server_update_backup_options(
        &self,
        server_id: &str,
        world_id: &str,
        options: HostingBackupOptionsPatch,
        acknowledge_rotation: bool,
    ) -> Result<HostingBackupOptionsUpdate, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to change Hosting backup settings."
                        .into(),
                )
            })?;
        let mut request = reqwest::Client::new()
            .patch(hosting_backup_url(
                server_id,
                world_id,
                &["backups-queue", "with-options"],
            )?)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session);
        if acknowledge_rotation {
            request = request.query(&[("ar", "true")]);
        }
        request
            .json(&options)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update Hosting backup settings: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update Hosting backup settings: {error}"
                )))
            })?
            .json::<HostingBackupOptionsUpdate>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting backup settings response: {error}"
                )))
            })
    }

    pub async fn hosting_server_create_backup(
        &self,
        server_id: &str,
        world_id: &str,
        name: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to create Hosting backups.".into(),
                )
            })?;
        let response = reqwest::Client::new()
            .post(hosting_backup_url(server_id, world_id, &["backups-queue"])?)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&serde_json::json!({"name": name.trim()}))
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not create Hosting backup: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not create Hosting backup: {error}"
                )))
            })?;
        drop(response);
        Ok(())
    }

    pub async fn hosting_server_restore_backup(
        &self,
        server_id: &str,
        world_id: &str,
        backup_id: &str,
        safety_backup_name: &str,
    ) -> Result<(), BackendError> {
        self.hosting_backup_write(
            server_id,
            world_id,
            &["backups-queue", backup_id, "restore"],
            reqwest::Method::POST,
            Some(serde_json::json!({"name": safety_backup_name})),
            "restore Hosting backup",
        )
        .await
    }

    pub async fn hosting_server_delete_backup(
        &self,
        server_id: &str,
        world_id: &str,
        backup_id: &str,
    ) -> Result<(), BackendError> {
        self.hosting_backup_write(
            server_id,
            world_id,
            &["backups-queue", backup_id],
            reqwest::Method::DELETE,
            None,
            "delete Hosting backup",
        )
        .await
    }

    pub async fn hosting_server_delete_backups(
        &self,
        server_id: &str,
        world_id: &str,
        backup_ids: &[String],
    ) -> Result<(), BackendError> {
        if backup_ids.is_empty() {
            return Err(BackendError::Configuration(
                "select at least one Hosting backup to delete".into(),
            ));
        }
        self.hosting_backup_write(
            server_id,
            world_id,
            &["backups-queue", "delete-many"],
            reqwest::Method::POST,
            Some(serde_json::json!({"backup_ids": backup_ids})),
            "delete Hosting backups",
        )
        .await
    }

    pub async fn hosting_server_undelete_backup(
        &self,
        server_id: &str,
        world_id: &str,
        backup_id: &str,
    ) -> Result<(), BackendError> {
        self.hosting_backup_write(
            server_id,
            world_id,
            &["backups-queue", backup_id, "undelete"],
            reqwest::Method::POST,
            None,
            "recover Hosting backup",
        )
        .await
    }

    pub async fn hosting_server_undelete_backups(
        &self,
        server_id: &str,
        world_id: &str,
        backup_ids: &[String],
    ) -> Vec<(String, Result<(), String>)> {
        futures::future::join_all(backup_ids.iter().map(
            |backup_id| async move {
                let result = self
                    .hosting_server_undelete_backup(
                        server_id, world_id, backup_id,
                    )
                    .await
                    .map_err(|error| error.to_string());
                (backup_id.clone(), result)
            },
        ))
        .await
    }

    pub async fn hosting_server_rename_backup(
        &self,
        server_id: &str,
        world_id: &str,
        backup_id: &str,
        name: &str,
    ) -> Result<(), BackendError> {
        self.hosting_backup_write(
            server_id,
            world_id,
            &["backups", backup_id],
            reqwest::Method::PATCH,
            Some(serde_json::json!({"name": name.trim()})),
            "rename Hosting backup",
        )
        .await
    }

    pub async fn hosting_server_download_backup(
        &self,
        server_id: &str,
        world_id: &str,
        backup_id: &str,
        destination: std::path::PathBuf,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to download Hosting backups.".into(),
                )
            })?;
        let client = reqwest::Client::new();
        let server = client
            .get(hosting_server_url(server_id)?)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, &credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting download information: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting download information: {error}"
                )))
            })?
            .json::<HostingNodeResponse>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting download information: {error}"
                )))
            })?;
        let node = server.node.ok_or_else(|| {
            BackendError::Operation(theseus::Error::msg(
                "this Hosting server has no assigned download node",
            ))
        })?;
        let authorization_url = hosting_node_backup_url(
            &node.instance,
            world_id,
            backup_id,
            true,
            None,
        )?;
        let authorization = client
            .get(authorization_url)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, &credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not authorize Hosting backup download: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not authorize Hosting backup download: {error}"
                )))
            })?
            .json::<HostingBackupDownloadAuthorization>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting download authorization: {error}"
                )))
            })?;
        let download_url = hosting_node_backup_url(
            &node.instance,
            world_id,
            backup_id,
            false,
            Some(&authorization.token),
        )?;
        let response = client
            .get(download_url)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not download Hosting backup: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not download Hosting backup: {error}"
                )))
            })?;
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let temp = tempfile::Builder::new()
            .prefix(".northstar-backup-")
            .tempfile_in(parent)
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not prepare backup destination: {error}"
                )))
            })?;
        let file = temp.reopen().map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not open temporary backup file: {error}"
            )))
        })?;
        let mut file = tokio::fs::File::from_std(file);
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "Hosting backup download was interrupted: {error}"
            )))
        })? {
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|error| {
                    BackendError::Operation(theseus::Error::msg(format!(
                        "could not write Hosting backup: {error}"
                    )))
                })?;
        }
        tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not finish Hosting backup: {error}"
                )))
            })?;
        file.sync_all().await.map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not finish Hosting backup: {error}"
            )))
        })?;
        drop(file);
        temp.persist(&destination).map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not save Hosting backup: {}",
                error.error
            )))
        })?;
        Ok(())
    }

    async fn hosting_backup_write(
        &self,
        server_id: &str,
        world_id: &str,
        tail: &[&str],
        method: reqwest::Method,
        body: Option<serde_json::Value>,
        action: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to manage Hosting backups.".into(),
                )
            })?;
        let url = hosting_backup_url(server_id, world_id, tail)?;
        let request = reqwest::Client::new()
            .request(method, url)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session);
        let request = if let Some(body) = body {
            request.json(&body)
        } else {
            request
        };
        request
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not {action}: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not {action}: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn hosting_server_access(
        &self,
        server_id: &str,
    ) -> Result<Vec<HostingServerAccessUser>, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to view Hosting access.".into(),
                )
            })?;
        reqwest::Client::new()
            .get(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}/users"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting access: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load Hosting access: {error}"
                )))
            })?
            .json::<Vec<HostingServerAccessUser>>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting access: {error}"
                )))
            })
    }

    pub async fn hosting_server_add_user(
        &self,
        server_id: &str,
        user_id: &str,
        role: HostingServerAccessRole,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to invite Hosting users.".into(),
                )
            })?;
        reqwest::Client::new()
            .post(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}/users"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&serde_json::json!({
                "user_id": user_id,
                "role": role.as_api_value(),
            }))
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not invite Hosting user: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not invite Hosting user: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn hosting_server_update_user_role(
        &self,
        server_id: &str,
        user_id: &str,
        role: HostingServerAccessRole,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to change Hosting access.".into(),
                )
            })?;
        reqwest::Client::new()
            .patch(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}/users/{user_id}"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&role.as_api_value())
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not change Hosting access role: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not change Hosting access role: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn hosting_server_reinvite_user(
        &self,
        server_id: &str,
        user_id: &str,
    ) -> Result<HostingServerReinviteResponse, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to resend Hosting invitations.".into(),
                )
            })?;
        reqwest::Client::new()
            .post(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}/users/{user_id}/reinvite"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not resend Hosting invitation: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not resend Hosting invitation: {error}"
                )))
            })?
            .json::<HostingServerReinviteResponse>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting invitation response: {error}"
                )))
            })
    }

    pub async fn hosting_server_remove_user(
        &self,
        server_id: &str,
        user_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to remove Hosting access.".into(),
                )
            })?;
        reqwest::Client::new()
            .delete(format!(
                "https://archon.modrinth.com/v1/servers/{server_id}/users/{user_id}"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not remove Hosting access: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not remove Hosting access: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn hosting_server_power(
        &self,
        server_id: &str,
        action: HostingPowerAction,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to manage Hosting servers.".into(),
                )
            })?;
        reqwest::Client::new()
            .post(format!(
                "https://archon.modrinth.com/modrinth/v0/servers/{server_id}/power"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&serde_json::json!({ "action": action.as_api_value() }))
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not send Hosting power action: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not send Hosting power action: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn connect_hosting_server(
        &self,
        server_id: &str,
    ) -> Result<
        (
            HostingServerConnection,
            tokio::sync::mpsc::UnboundedReceiver<HostingServerEvent>,
        ),
        BackendError,
    > {
        use async_tungstenite::tungstenite::Message;
        use async_tungstenite::tungstenite::client::IntoClientRequest;
        use futures::StreamExt;

        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to view the Hosting console.".into(),
                )
            })?;
        let auth = reqwest::Client::new()
            .get(format!(
                "https://archon.modrinth.com/modrinth/v0/servers/{server_id}/ws"
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not authorize the Hosting console: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not authorize the Hosting console: {error}"
                )))
            })?
            .json::<HostingWebSocketAuth>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read Hosting console authorization: {error}"
                )))
            })?;
        let socket_url = hosting_websocket_url(&auth.url);
        let request = socket_url.into_client_request().map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "invalid Hosting console websocket URL: {error}"
            )))
        })?;
        let (mut socket, _) = async_tungstenite::tokio::connect_async(request)
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not connect to the Hosting console: {error}"
                )))
            })?;
        let authentication = serde_json::json!({
            "event": "auth",
            "jwt": auth.token,
        })
        .to_string();
        socket
            .send(Message::Text(authentication.into()))
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not authenticate the Hosting console: {error}"
                )))
            })?;
        let authenticated =
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                while let Some(message) = socket.next().await {
                    match message.map_err(|error| error.to_string())? {
                        Message::Text(text) => {
                            let event =
                                serde_json::from_str::<serde_json::Value>(
                                    &text,
                                )
                                .map_err(|error| error.to_string())?;
                            match event
                                .get("event")
                                .and_then(|value| value.as_str())
                            {
                                Some("auth-ok") => {
                                    return Ok::<bool, String>(true);
                                }
                                Some("auth-incorrect") => {
                                    return Ok::<bool, String>(false);
                                }
                                _ => {}
                            }
                        }
                        Message::Ping(payload) => socket
                            .send(Message::Pong(payload))
                            .await
                            .map_err(|error| error.to_string())?,
                        Message::Close(_) => return Ok::<bool, String>(false),
                        _ => {}
                    }
                }
                Ok::<bool, String>(false)
            })
            .await
            .map_err(|_| {
                BackendError::Operation(theseus::Error::msg(
                    "Hosting console authentication timed out.",
                ))
            })?
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "Hosting console authentication failed: {error}"
                )))
            })?;
        if !authenticated {
            return Err(BackendError::Authentication(
                "The Hosting console rejected its authorization token.".into(),
            ));
        }

        let (mut socket_sink, mut socket_stream) = socket.split();
        let (event_sender, event_receiver) =
            tokio::sync::mpsc::unbounded_channel();
        let (command_sender, mut command_receiver) =
            tokio::sync::mpsc::unbounded_channel::<String>();
        let (shutdown_sender, mut shutdown_receiver) =
            tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_receiver => break,
                    command = command_receiver.recv() => match command {
                        Some(command) => {
                            let payload = serde_json::json!({ "event": "command", "cmd": command }).to_string();
                            if socket_sink.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
                    incoming = socket_stream.next() => match incoming {
                        Some(Ok(Message::Text(text))) => {
                            if let Some(event) = parse_hosting_server_event(&text) {
                                if event_sender.send(event).is_err() { break; }
                            }
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            if socket_sink.send(Message::Pong(payload)).await.is_err() { break; }
                        }
                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                        _ => {}
                    },
                }
            }
            let _ = event_sender.send(HostingServerEvent::Disconnected);
        });
        Ok((
            HostingServerConnection {
                commands: command_sender,
                shutdown: Some(shutdown_sender),
            },
            event_receiver,
        ))
    }

    /// Saves a complete settings object. Prefer `update_settings` for edits.
    pub async fn set_settings(
        &self,
        settings: theseus::data::Settings,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let _guard = SETTINGS_UPDATE_LOCK.lock().await;
        theseus::settings::set(settings)
            .await
            .map_err(BackendError::Operation)
    }

    /// Applies an edit to the latest settings, serialized with other adapter edits.
    /// Writers that bypass this adapter are not covered by this lock.
    pub async fn update_settings<F>(&self, edit: F) -> Result<(), BackendError>
    where
        F: FnOnce(&mut theseus::data::Settings),
    {
        self.wait_ready().await?;
        let _guard = SETTINGS_UPDATE_LOCK.lock().await;
        let mut settings = theseus::settings::get()
            .await
            .map_err(BackendError::Operation)?;
        edit(&mut settings);
        theseus::settings::set(settings)
            .await
            .map_err(BackendError::Operation)
    }

    /// Loads the native-only recent-item limit without changing Theseus settings.
    pub async fn quick_instance_count(&self) -> Result<usize, BackendError> {
        self.wait_ready().await?;
        let path = native_ui_preferences_path()?;
        let contents = match tokio::fs::read(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DEFAULT_QUICK_INSTANCE_COUNT);
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        let preferences: serde_json::Value = serde_json::from_slice(&contents)
            .map_err(|error| {
                BackendError::Configuration(format!(
                    "could not parse native UI preferences: {error}"
                ))
            })?;
        Ok(preferences
            .get("quick_instance_count")
            .and_then(serde_json::Value::as_u64)
            .map(|count| (count as usize).clamp(1, MAX_QUICK_INSTANCE_COUNT))
            .unwrap_or(DEFAULT_QUICK_INSTANCE_COUNT))
    }

    /// Persists the native home screen's recent-item limit independently of
    /// upstream Theseus settings, matching the upstream app's local setting.
    pub async fn set_quick_instance_count(
        &self,
        count: usize,
    ) -> Result<usize, BackendError> {
        self.wait_ready().await?;
        let count = count.clamp(1, MAX_QUICK_INSTANCE_COUNT);
        let _guard = NATIVE_UI_PREFERENCES_LOCK.lock().await;
        let path = native_ui_preferences_path()?;
        let mut preferences = match tokio::fs::read(&path).await {
            Ok(contents) => {
                serde_json::from_slice::<serde_json::Value>(&contents)
                    .unwrap_or_else(|_| serde_json::json!({}))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_json::json!({})
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        if !preferences.is_object() {
            preferences = serde_json::json!({});
        }
        preferences["quick_instance_count"] = serde_json::json!(count);
        let contents =
            serde_json::to_vec_pretty(&preferences).map_err(|error| {
                BackendError::Configuration(format!(
                    "could not encode native UI preferences: {error}"
                ))
            })?;
        tokio::fs::write(&path, contents).await.map_err(|error| {
            BackendError::Configuration(format!(
                "could not save native UI preferences: {error}"
            ))
        })?;
        Ok(count)
    }

    pub async fn native_color_theme(&self) -> Result<u8, BackendError> {
        self.wait_ready().await?;
        let path = native_ui_preferences_path()?;
        let contents = match tokio::fs::read(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(0);
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        let preferences: serde_json::Value = serde_json::from_slice(&contents)
            .map_err(|error| {
                BackendError::Configuration(format!(
                    "could not parse native UI preferences: {error}"
                ))
            })?;
        Ok(preferences
            .get("color_theme")
            .and_then(serde_json::Value::as_u64)
            .map(|theme| clamp_native_color_theme(theme as u8))
            .unwrap_or(0))
    }

    pub async fn set_native_color_theme(
        &self,
        theme: u8,
    ) -> Result<u8, BackendError> {
        self.wait_ready().await?;
        let theme = clamp_native_color_theme(theme);
        let _guard = NATIVE_UI_PREFERENCES_LOCK.lock().await;
        let path = native_ui_preferences_path()?;
        let mut preferences = match tokio::fs::read(&path).await {
            Ok(contents) => {
                serde_json::from_slice::<serde_json::Value>(&contents)
                    .unwrap_or_else(|_| serde_json::json!({}))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_json::json!({})
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        if !preferences.is_object() {
            preferences = serde_json::json!({});
        }
        preferences["color_theme"] = serde_json::json!(theme);
        let contents =
            serde_json::to_vec_pretty(&preferences).map_err(|error| {
                BackendError::Configuration(format!(
                    "could not encode native UI preferences: {error}"
                ))
            })?;
        tokio::fs::write(&path, contents).await.map_err(|error| {
            BackendError::Configuration(format!(
                "could not save native UI preferences: {error}"
            ))
        })?;
        Ok(theme)
    }

    pub async fn preferred_dark_theme(
        &self,
    ) -> Result<theseus::data::Theme, BackendError> {
        self.wait_ready().await?;
        let path = native_ui_preferences_path()?;
        let contents = match tokio::fs::read(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(theseus::data::Theme::Dark);
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        let preferences: serde_json::Value = serde_json::from_slice(&contents)
            .map_err(|error| {
                BackendError::Configuration(format!(
                    "could not parse native UI preferences: {error}"
                ))
            })?;
        Ok(preferred_dark_theme_from_preferences(&preferences))
    }

    pub async fn set_preferred_dark_theme(
        &self,
        theme: theseus::data::Theme,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let _guard = NATIVE_UI_PREFERENCES_LOCK.lock().await;
        let path = native_ui_preferences_path()?;
        let mut preferences = match tokio::fs::read(&path).await {
            Ok(contents) => {
                serde_json::from_slice::<serde_json::Value>(&contents)
                    .unwrap_or_else(|_| serde_json::json!({}))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_json::json!({})
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        if !preferences.is_object() {
            preferences = serde_json::json!({});
        }
        preferences["preferred_dark_theme"] =
            serde_json::json!(preferred_dark_theme_name(theme));
        let contents =
            serde_json::to_vec_pretty(&preferences).map_err(|error| {
                BackendError::Configuration(format!(
                    "could not encode native UI preferences: {error}"
                ))
            })?;
        tokio::fs::write(&path, contents).await.map_err(|error| {
            BackendError::Configuration(format!(
                "could not save native UI preferences: {error}"
            ))
        })
    }

    pub async fn theme_sync_across_devices(
        &self,
    ) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        let path = native_ui_preferences_path()?;
        let contents = match tokio::fs::read(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false);
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        let preferences: serde_json::Value = serde_json::from_slice(&contents)
            .map_err(|error| {
                BackendError::Configuration(format!(
                    "could not parse native UI preferences: {error}"
                ))
            })?;
        Ok(native_theme_sync_enabled(&preferences))
    }

    pub async fn set_theme_sync_across_devices(
        &self,
        enabled: bool,
    ) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        let _guard = NATIVE_UI_PREFERENCES_LOCK.lock().await;
        let path = native_ui_preferences_path()?;
        let mut preferences = match tokio::fs::read(&path).await {
            Ok(contents) => {
                serde_json::from_slice::<serde_json::Value>(&contents)
                    .unwrap_or_else(|_| serde_json::json!({}))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_json::json!({})
            }
            Err(error) => {
                return Err(BackendError::Configuration(format!(
                    "could not read native UI preferences: {error}"
                )));
            }
        };
        if !preferences.is_object() {
            preferences = serde_json::json!({});
        }
        preferences["sync_theme_across_devices"] = serde_json::json!(enabled);
        let contents =
            serde_json::to_vec_pretty(&preferences).map_err(|error| {
                BackendError::Configuration(format!(
                    "could not encode native UI preferences: {error}"
                ))
            })?;
        tokio::fs::write(&path, contents).await.map_err(|error| {
            BackendError::Configuration(format!(
                "could not save native UI preferences: {error}"
            ))
        })?;
        Ok(enabled)
    }

    pub async fn user_preferences(
        &self,
        user_id: &str,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        theseus::users::get_user_preferences(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_user_appearance_theme(
        &self,
        user_id: &str,
        theme: theseus::data::Theme,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        let appearance = if matches!(theme, theseus::data::Theme::System) {
            serde_json::json!({ "auto": true })
        } else {
            serde_json::json!({ "auto": false, "theme": theme.as_str() })
        };
        theseus::users::patch_user_preferences(
            user_id,
            serde_json::json!({ "appearance": appearance }),
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn get_instance(
        &self,
        id: &str,
    ) -> Result<Option<theseus::data::InstanceMetadata>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn can_use_shared_instances(&self) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        theseus::instance::can_active_user_use_shared_instances()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn shared_instance_users(
        &self,
        instance_id: &str,
    ) -> Result<theseus::instance::SharedInstanceUsers, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_shared_instance_users(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn search_users(
        &self,
        query: &str,
    ) -> Result<Vec<theseus::users::SearchUser>, BackendError> {
        self.wait_ready().await?;
        theseus::users::search_user(query)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn user_profile(
        &self,
        user_id: &str,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        theseus::users::get_user_profile(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn user_projects(
        &self,
        user_id: &str,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        theseus::users::get_user_projects(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn user_organizations(
        &self,
        user_id: &str,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        theseus::users::get_user_organizations(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn user_collections(
        &self,
        user_id: &str,
    ) -> Result<serde_json::Value, BackendError> {
        self.wait_ready().await?;
        theseus::users::get_user_collections(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn friends(&self) -> Result<Vec<NativeFriend>, BackendError> {
        self.wait_ready().await?;
        let Some(credentials) = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
        else {
            return Ok(Vec::new());
        };
        let friends = theseus::friends::friends()
            .await
            .map_err(BackendError::Operation)?;
        if friends.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids = friends
            .iter()
            .map(|friend| {
                if friend.id == credentials.user_id {
                    friend.friend_id.as_str()
                } else {
                    friend.id.as_str()
                }
            })
            .collect::<Vec<_>>();
        let (users, statuses) = tokio::try_join!(
            theseus::cache::get_user_many(&user_ids, None),
            theseus::friends::friend_statuses(),
        )
        .map_err(BackendError::Operation)?;
        Ok(friends
            .into_iter()
            .map(|friend| {
                let user_id = if friend.id == credentials.user_id {
                    friend.friend_id.clone()
                } else {
                    friend.id.clone()
                };
                let user = users.iter().find(|user| user.id == user_id);
                let status = statuses
                    .iter()
                    .find(|status| status.user_id.to_string() == user_id);
                NativeFriend {
                    user_id,
                    username: user
                        .map(|user| user.username.clone())
                        .unwrap_or_else(|| "Modrinth user".into()),
                    avatar_url: user.and_then(|user| user.avatar_url.clone()),
                    accepted: friend.accepted,
                    incoming_request: !friend.accepted
                        && friend.id == credentials.user_id,
                    online: status.is_some(),
                    playing_on: status
                        .and_then(|status| status.profile_name.clone()),
                    last_updated: status
                        .map(|status| status.last_update.timestamp_millis()),
                    created: friend.created.timestamp(),
                }
            })
            .collect())
    }

    pub async fn add_friend(&self, user_id: &str) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::friends::add_friend(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn remove_friend(
        &self,
        user_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::friends::remove_friend(user_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn invite_shared_instance_users(
        &self,
        instance_id: &str,
        user_ids: Vec<String>,
    ) -> Result<theseus::instance::SharedInstanceUsers, BackendError> {
        self.wait_ready().await?;
        theseus::instance::invite_shared_instance_users(instance_id, user_ids)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn create_shared_instance_invite_link(
        &self,
        instance_id: &str,
        max_age_seconds: Option<i32>,
        max_uses: Option<i32>,
        replace_invite_id: Option<String>,
    ) -> Result<theseus::instance::SharedInstanceInviteLink, BackendError> {
        self.wait_ready().await?;
        theseus::instance::create_shared_instance_invite_link(
            instance_id,
            max_age_seconds,
            max_uses,
            replace_invite_id,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn shared_instance_invites(
        &self,
        instance_id: &str,
    ) -> Result<Vec<theseus::instance::SharedInstanceInvite>, BackendError>
    {
        self.wait_ready().await?;
        theseus::instance::get_shared_instance_invites(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn revoke_shared_instance_invite(
        &self,
        instance_id: &str,
        invite_id: String,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::revoke_shared_instance_invite(instance_id, invite_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn remove_shared_instance_users(
        &self,
        instance_id: &str,
        user_ids: Vec<String>,
        has_pending_recipients: bool,
    ) -> Result<theseus::instance::SharedInstanceUsers, BackendError> {
        self.wait_ready().await?;
        theseus::instance::remove_shared_instance_users(
            instance_id,
            user_ids,
            has_pending_recipients,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn shared_instance_publish_preview(
        &self,
        instance_id: &str,
    ) -> Result<
        Option<theseus::instance::SharedInstancePublishPreview>,
        BackendError,
    > {
        self.wait_ready().await?;
        theseus::instance::get_shared_instance_publish_preview(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn publish_shared_instance(
        &self,
        instance_id: &str,
        file_paths: Vec<String>,
        on_progress: impl Fn(u64, u64) + Send + Sync,
    ) -> Result<theseus::data::SharedInstanceAttachment, BackendError> {
        self.wait_ready().await?;
        theseus::instance::publish_shared_instance(
            instance_id,
            file_paths,
            &|uploaded, total| on_progress(uploaded, total),
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn shared_instance_publish_support_details(
        &self,
        instance_id: &str,
    ) -> Result<Option<String>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_shared_instance_publish_support_details(
            instance_id,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn unlink_shared_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::unlink_shared_instance(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn shared_instance_relink_preview(
        &self,
        instance_id: &str,
        input: &str,
    ) -> Result<theseus::instance::SharedInstanceRelinkPreview, BackendError>
    {
        self.wait_ready().await?;
        theseus::instance::get_shared_instance_relink_preview(
            instance_id,
            input,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn relink_shared_instance(
        &self,
        instance_id: &str,
        shared_instance_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::relink_shared_instance(
            instance_id,
            shared_instance_id,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn unpublish_shared_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::unpublish_shared_instance(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn unpublish_remote_shared_instance(
        &self,
        shared_instance_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::unpublish_remote_shared_instance(shared_instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn cached_shared_instance_library(
        &self,
    ) -> Result<Vec<theseus::data::CachedSharedInstanceSummary>, BackendError>
    {
        self.wait_ready().await?;
        theseus::instance::get_cached_shared_instance_library()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn instance_worlds(
        &self,
        instance_id: &str,
    ) -> Result<Vec<worlds::World>, BackendError> {
        self.wait_ready().await?;
        worlds::get_instance_worlds(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn server_status(
        &self,
        address: &str,
    ) -> Result<worlds::ServerStatus, BackendError> {
        self.wait_ready().await?;
        worlds::get_server_status(address, None)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_world_display_status(
        &self,
        instance_id: &str,
        world_type: worlds::WorldType,
        world_id: &str,
        status: worlds::DisplayStatus,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        worlds::set_world_display_status(
            instance_id,
            world_type,
            world_id,
            status,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn add_server_to_instance(
        &self,
        instance_id: &str,
        name: String,
        address: String,
        pack_status: worlds::ServerPackStatus,
    ) -> Result<usize, BackendError> {
        self.wait_ready().await?;
        worlds::add_server_to_instance(
            instance_id,
            name,
            address,
            pack_status,
            None,
            None,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn edit_server_in_instance(
        &self,
        instance_id: &str,
        index: usize,
        name: String,
        address: String,
        pack_status: worlds::ServerPackStatus,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        worlds::edit_server_in_instance(
            instance_id,
            index,
            name,
            address,
            pack_status,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn remove_server_from_instance(
        &self,
        instance_id: &str,
        index: usize,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        worlds::remove_server_from_instance(instance_id, index)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn rename_singleplayer_world(
        &self,
        instance_id: &str,
        world_id: &str,
        name: &str,
        reset_icon: bool,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        if !valid_world_component(world_id) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "invalid world identifier",
            )));
        }
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
        {
            return Err(BackendError::Operation(theseus::Error::msg(
                "world names must contain 1 to 64 non-control characters",
            )));
        }
        let world =
            theseus::worlds::get_singleplayer_world(instance_id, world_id)
                .await
                .map_err(BackendError::Operation)?;
        if !matches!(
            &world.details,
            worlds::WorldDetails::Singleplayer { locked: false, .. }
        ) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "world is open or is not a singleplayer save",
            )));
        }
        let instance_path = theseus::instance::get_full_path(instance_id)
            .await
            .map_err(BackendError::Operation)?;
        theseus::worlds::rename_world(&instance_path, world_id, name)
            .await
            .map_err(BackendError::Operation)?;
        if reset_icon {
            theseus::worlds::reset_world_icon(&instance_path, world_id)
                .await
                .map_err(BackendError::Operation)?;
        }
        Ok(())
    }

    pub async fn delete_singleplayer_world(
        &self,
        instance_id: &str,
        world_id: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        if !valid_world_component(world_id) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "invalid world identifier",
            )));
        }
        let world =
            theseus::worlds::get_singleplayer_world(instance_id, world_id)
                .await
                .map_err(BackendError::Operation)?;
        if !matches!(
            &world.details,
            worlds::WorldDetails::Singleplayer { locked: false, .. }
        ) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "world is open or is not a singleplayer save",
            )));
        }
        let instance_path = theseus::instance::get_full_path(instance_id)
            .await
            .map_err(BackendError::Operation)?;
        theseus::worlds::delete_world(&instance_path, world_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn screenshots(
        &self,
        instance_id: Option<&str>,
    ) -> Result<Vec<theseus::instance::InstanceScreenshot>, BackendError> {
        self.wait_ready().await?;
        match instance_id {
            Some(instance_id) => {
                theseus::instance::list_screenshots(instance_id)
                    .await
                    .map_err(BackendError::Operation)
            }
            None => theseus::instance::list_all_screenshots()
                .await
                .map_err(BackendError::Operation),
        }
    }

    pub async fn delete_screenshot(
        &self,
        key: theseus::instance::ScreenshotKey,
    ) -> Result<(), BackendError> {
        self.delete_screenshots(vec![key]).await
    }

    pub async fn delete_screenshots(
        &self,
        keys: Vec<theseus::instance::ScreenshotKey>,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::delete_screenshots(&keys)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn export_screenshot(
        &self,
        key: theseus::instance::ScreenshotKey,
        destination: std::path::PathBuf,
    ) -> Result<(), BackendError> {
        self.export_screenshots(vec![key], destination).await
    }

    pub async fn export_screenshots(
        &self,
        keys: Vec<theseus::instance::ScreenshotKey>,
        destination: std::path::PathBuf,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::export_screenshots(&keys, destination)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn screenshot_groups(
        &self,
    ) -> Result<Vec<theseus::instance::ScreenshotGroup>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::list_screenshot_groups()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn create_screenshot_group(
        &self,
        name: String,
        screenshot_ids: Vec<String>,
    ) -> Result<theseus::instance::ScreenshotGroup, BackendError> {
        self.wait_ready().await?;
        theseus::instance::create_screenshot_group(name, screenshot_ids)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn rename_screenshot_group(
        &self,
        id: String,
        name: String,
    ) -> Result<theseus::instance::ScreenshotGroup, BackendError> {
        self.wait_ready().await?;
        theseus::instance::rename_screenshot_group(id, name)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn delete_screenshot_group(
        &self,
        id: String,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::delete_screenshot_group(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_screenshot_group_membership(
        &self,
        screenshot_id: String,
        group_id: Option<String>,
    ) -> Result<(), BackendError> {
        self.set_screenshot_group_memberships(vec![(screenshot_id, group_id)])
            .await
    }

    pub async fn set_screenshot_group_memberships(
        &self,
        updates: Vec<(String, Option<String>)>,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::set_screenshot_group_memberships(
            updates
                .into_iter()
                .map(|(screenshot_id, group_id)| {
                    theseus::instance::ScreenshotGroupMembershipUpdate {
                        screenshot_id,
                        group_id,
                    }
                })
                .collect(),
        )
        .await
        .map_err(BackendError::Operation)
    }

    /// Searches using the same encoded query string and cache as the existing app.
    pub async fn search_projects(
        &self,
        query: &str,
        cache: Option<theseus::data::CacheBehaviour>,
    ) -> Result<Option<theseus::data::SearchResultsV3>, BackendError> {
        self.wait_ready().await?;
        theseus::cache::get_search_results_v3(query, cache)
            .await
            .map_err(BackendError::Operation)
    }

    /// Fetches the same public article feed used by the official app sidebar.
    /// News is independent of Theseus readiness and can load alongside startup.
    pub async fn news_articles(&self) -> Result<Vec<NewsArticle>, String> {
        const FEED_URL: &str = "https://modrinth.com/news/feed/articles.json";
        const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

        let response = reqwest::get(FEED_URL).await.map_err(|error| {
            format!("Could not load Modrinth news: {error}")
        })?;
        if !response.status().is_success() {
            return Err(format!(
                "Could not load Modrinth news: HTTP {}",
                response.status()
            ));
        }
        let bytes = response.bytes().await.map_err(|error| {
            format!("Could not read Modrinth news: {error}")
        })?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err("The Modrinth news feed was unexpectedly large.".into());
        }
        let feed: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| {
                format!("Could not parse Modrinth news: {error}")
            })?;
        let Some(articles) = feed.get("articles").and_then(|v| v.as_array())
        else {
            return Err("The Modrinth news feed has no articles list.".into());
        };

        let mut result = Vec::with_capacity(4);
        for article in articles {
            let value = |key: &str| {
                article
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            let title = value("title");
            let thumbnail = value("thumbnail");
            let summary = value("summary");
            let date = value("date");
            let link = value("link");
            let valid_link = url::Url::parse(&link).is_ok_and(|url| {
                url.scheme() == "https"
                    && matches!(
                        url.host_str(),
                        Some("modrinth.com" | "www.modrinth.com")
                    )
            });
            let valid_thumbnail = url::Url::parse(&thumbnail)
                .is_ok_and(|url| url.scheme() == "https");
            if title.is_empty()
                || !valid_link
                || !valid_thumbnail
                || date.is_empty()
            {
                continue;
            }
            result.push(NewsArticle {
                title,
                thumbnail,
                summary,
                date,
                link,
            });
            if result.len() == 4 {
                break;
            }
        }
        Ok(result)
    }

    pub async fn project(
        &self,
        id: &str,
        cache: Option<theseus::data::CacheBehaviour>,
    ) -> Result<Option<theseus::data::ProjectV3>, BackendError> {
        self.wait_ready().await?;
        theseus::cache::get_project_v3(id, cache)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn project_is_followed(
        &self,
        project_id: &str,
    ) -> Result<bool, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to follow projects.".into(),
                )
            })?;
        let response = reqwest::Client::new()
            .get(format!(
                "https://api.modrinth.com/v3/user/{}/follows",
                credentials.user_id
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load followed projects: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load followed projects: {error}"
                )))
            })?
            .json::<Vec<serde_json::Value>>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read followed projects: {error}"
                )))
            })?;
        Ok(response.iter().any(|project| {
            project.get("id").and_then(serde_json::Value::as_str)
                == Some(project_id)
        }))
    }

    pub async fn set_project_followed(
        &self,
        project_id: &str,
        followed: bool,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to follow projects.".into(),
                )
            })?;
        let url = project_follow_url(project_id)?;
        let client = reqwest::Client::new();
        let request = if followed {
            client.post(url)
        } else {
            client.delete(url)
        };
        request
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update project follow: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update project follow: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn content_service_collections(
        &self,
    ) -> Result<Vec<ContentServiceCollection>, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to save projects to collections."
                        .into(),
                )
            })?;
        reqwest::Client::new()
            .get(format!(
                "https://api.modrinth.com/v3/user/{}/collections",
                credentials.user_id
            ))
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load collections: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not load collections: {error}"
                )))
            })?
            .json::<Vec<ContentServiceCollection>>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read collections: {error}"
                )))
            })
    }

    pub async fn set_collection_projects(
        &self,
        collection_id: &str,
        project_ids: &[String],
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to save projects to collections."
                        .into(),
                )
            })?;
        let url = collection_url(collection_id)?;
        reqwest::Client::new()
            .patch(url)
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&serde_json::json!({ "new_projects": project_ids }))
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update collection: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not update collection: {error}"
                )))
            })?;
        Ok(())
    }

    pub async fn create_content_service_collection(
        &self,
        name: &str,
        initial_project_id: &str,
    ) -> Result<ContentServiceCollection, BackendError> {
        self.wait_ready().await?;
        let credentials = theseus::mr_auth::get_credentials()
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Authentication(
                    "Sign in to Modrinth to create a collection.".into(),
                )
            })?;
        reqwest::Client::new()
            .post("https://api.modrinth.com/v3/collection")
            .header(reqwest::header::USER_AGENT, theseus::launcher_user_agent())
            .header(reqwest::header::AUTHORIZATION, credentials.session)
            .json(&serde_json::json!({
                "name": name.trim(),
                "projects": [initial_project_id],
            }))
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not create collection: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not create collection: {error}"
                )))
            })?
            .json::<ContentServiceCollection>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read created collection: {error}"
                )))
            })
    }

    pub async fn project_versions(
        &self,
        id: &str,
        cache: Option<theseus::data::CacheBehaviour>,
    ) -> Result<Option<Vec<theseus::data::Version>>, BackendError> {
        self.wait_ready().await?;
        theseus::cache::get_project_versions(id, cache)
            .await
            .map_err(BackendError::Operation)
    }

    /// Resolves dependencies and starts the installation in the backend.
    /// Await the task for its outcome even if the event stream lags.
    pub async fn install_project_with_dependencies(
        &self,
        instance_id: &str,
        request: theseus::instance::InstallProjectWithDependenciesRequest,
    ) -> Result<ContentInstallation, BackendError> {
        self.wait_ready().await?;
        let (plan, task) =
            theseus::instance::install_project_with_dependencies_tracked(
                instance_id,
                request,
            )
            .await
            .map_err(BackendError::Operation)?;
        Ok(ContentInstallation { plan, task })
    }

    pub async fn instance_content(
        &self,
        instance_id: &str,
        cache: Option<theseus::data::CacheBehaviour>,
    ) -> Result<Vec<theseus::data::ContentItem>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_content_items(instance_id, cache)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn installed_project_ids(
        &self,
        instance_id: &str,
    ) -> Result<Vec<String>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_installed_project_ids(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn add_content_file(
        &self,
        instance_id: &str,
        path: &std::path::Path,
        project_type: Option<theseus::data::ProjectType>,
    ) -> Result<String, BackendError> {
        self.wait_ready().await?;
        theseus::instance::add_project_from_path(
            instance_id,
            path,
            project_type,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn set_content_enabled(
        &self,
        instance_id: &str,
        project_path: &str,
        enabled: bool,
    ) -> Result<String, BackendError> {
        self.wait_ready().await?;
        theseus::instance::toggle_disable_project(
            instance_id,
            project_path,
            Some(enabled),
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn remove_content(
        &self,
        instance_id: &str,
        project_path: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::remove_project(instance_id, project_path)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn update_content(
        &self,
        instance_id: &str,
        project_path: &str,
    ) -> Result<String, BackendError> {
        self.wait_ready().await?;
        theseus::instance::update_project(instance_id, project_path, None)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn export_candidates(
        &self,
        instance_id: &str,
    ) -> Result<Vec<theseus::instance::PackExportCandidate>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::get_pack_export_candidates(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    /// Exports through a temporary file without replacing an existing destination.
    pub async fn export_pack(
        &self,
        request: ExportPack,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        let parent = request
            .destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| {
                BackendError::Operation(theseus::Error::msg(
                    "export destination requires a parent directory",
                ))
            })?
            .to_path_buf();
        let temporary = tokio::task::spawn_blocking(move || {
            tempfile::NamedTempFile::new_in(parent)
                .map(|file| file.into_temp_path())
        })
        .await
        .map_err(|error| BackendError::Operation(error.into()))?
        .map_err(|error| BackendError::Operation(error.into()))?;

        theseus::instance::export_mrpack(
            &request.instance_id,
            temporary.to_path_buf(),
            request.included_paths,
            request.excluded_paths,
            request.version_id,
            request.description,
            request.name,
        )
        .await
        .map_err(BackendError::Operation)?;

        tokio::task::spawn_blocking(move || {
            temporary.persist_noclobber(request.destination)
        })
        .await
        .map_err(|error| BackendError::Operation(error.into()))?
        .map_err(|error| BackendError::Operation(error.error.into()))?;
        Ok(())
    }

    pub async fn edit_instance(
        &self,
        id: &str,
        patch: theseus::data::EditInstance,
    ) -> Result<theseus::data::InstanceMetadata, BackendError> {
        self.wait_ready().await?;
        theseus::instance::edit(id, patch)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn list_instance_files(
        &self,
        instance_id: &str,
        relative_path: &str,
    ) -> Result<Vec<theseus::instance::InstanceFileItem>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::list_instance_files(instance_id, relative_path)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn read_instance_file(
        &self,
        instance_id: &str,
        relative_path: &str,
    ) -> Result<Vec<u8>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::read_instance_file(instance_id, relative_path)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn write_instance_file(
        &self,
        instance_id: &str,
        relative_path: &str,
        bytes: &[u8],
        create_only: bool,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::write_instance_file(
            instance_id,
            relative_path,
            bytes,
            create_only,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn create_instance_directory(
        &self,
        instance_id: &str,
        relative_path: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::create_instance_directory(instance_id, relative_path)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn rename_instance_file(
        &self,
        instance_id: &str,
        source: &str,
        destination: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::rename_instance_file(
            instance_id,
            source,
            destination,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn delete_instance_file(
        &self,
        instance_id: &str,
        relative_path: &str,
        recursive: bool,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::delete_instance_file(
            instance_id,
            relative_path,
            recursive,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn running_processes(
        &self,
    ) -> Result<Vec<theseus::data::ProcessMetadata>, BackendError> {
        self.wait_ready().await?;
        theseus::process::get_all()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn stop_process(
        &self,
        id: uuid::Uuid,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::process::kill(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn instance_logs(
        &self,
        instance_id: &str,
    ) -> Result<Vec<theseus::logs::Logs>, BackendError> {
        self.wait_ready().await?;
        theseus::logs::get_logs(instance_id, Some(true))
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn instance_log_content(
        &self,
        instance_id: &str,
        log_type: theseus::logs::LogType,
        filename: &str,
    ) -> Result<String, BackendError> {
        self.wait_ready().await?;
        if !valid_log_filename(filename) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "log filename must be a single file name",
            )));
        }
        theseus::logs::get_output_by_filename(instance_id, log_type, filename)
            .await
            .map(|output| output.as_str().to_owned())
            .map_err(BackendError::Operation)
    }

    pub async fn delete_instance_log(
        &self,
        instance_id: &str,
        log_type: theseus::logs::LogType,
        filename: &str,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        if !valid_log_filename(filename) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "log filename must be a single file name",
            )));
        }
        if filename == "latest.log"
            && theseus::process::get_all()
                .await
                .map_err(BackendError::Operation)?
                .iter()
                .any(|process| process.instance_id == instance_id)
        {
            return Err(BackendError::Operation(theseus::Error::msg(
                "cannot delete latest.log while the instance is running",
            )));
        }
        theseus::logs::delete_logs_by_filename(instance_id, log_type, filename)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn live_log(
        &self,
        instance_id: &str,
    ) -> Result<theseus::logs::CensoredString, BackendError> {
        self.wait_ready().await?;
        theseus::logs::get_live_log_buffer(instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    /// Clear the in-memory live log buffer for an instance.
    pub fn clear_live_log_buffer(&self, instance_id: &str) {
        theseus::logs::clear_live_log_buffer(instance_id);
    }

    /// Analyze user-selected Minecraft log text with mclo.gs Insights.
    /// Log contents are sent only when the caller explicitly requests analysis.
    pub async fn analyse_minecraft_log(
        &self,
        content: &str,
    ) -> Result<serde_json::Value, BackendError> {
        let response = mclogs_client()?
            .post("https://api.mclo.gs/1/analyse")
            .form(&[("content", content)])
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not reach mclo.gs crash insights: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "mclo.gs crash insights returned an error: {error}"
                )))
            })?;
        response.json().await.map_err(|error| {
            BackendError::Operation(theseus::Error::msg(format!(
                "could not read mclo.gs crash insights: {error}"
            )))
        })
    }

    /// Upload log text to mclo.gs after an explicit user share action.
    pub async fn share_minecraft_log(
        &self,
        content: &str,
    ) -> Result<String, BackendError> {
        let response = mclogs_client()?
            .post("https://api.mclo.gs/1/log")
            .form(&[("content", content)])
            .send()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not reach mclo.gs log sharing: {error}"
                )))
            })?
            .error_for_status()
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "mclo.gs log sharing returned an error: {error}"
                )))
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|error| {
                BackendError::Operation(theseus::Error::msg(format!(
                    "could not read mclo.gs share response: {error}"
                )))
            })?;
        mclogs_share_url(&response)
    }

    pub async fn latest_log_cursor(
        &self,
        instance_id: &str,
        cursor: u64,
    ) -> Result<theseus::logs::LatestLogCursor, BackendError> {
        self.wait_ready().await?;
        theseus::logs::get_latest_log_cursor(instance_id, cursor)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn minecraft_versions(
        &self,
    ) -> Result<theseus::metadata::VersionManifest, BackendError> {
        self.wait_ready().await?;
        theseus::metadata::get_minecraft_versions()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn loader_versions(
        &self,
        loader: theseus::data::ModLoader,
    ) -> Result<theseus::metadata::Manifest, BackendError> {
        self.wait_ready().await?;
        if loader == theseus::data::ModLoader::Vanilla {
            return Err(BackendError::Operation(theseus::Error::msg(
                "vanilla Minecraft has no mod loader manifest",
            )));
        }
        theseus::metadata::get_loader_versions(loader.as_meta_str())
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn configured_java_versions(
        &self,
    ) -> Result<Vec<theseus::data::JavaVersion>, BackendError> {
        self.wait_ready().await?;
        let versions = theseus::jre::get_java_versions()
            .await
            .map_err(BackendError::Operation)?;
        let mut result: Vec<_> =
            versions.into_iter().map(|(_, version)| version).collect();
        result.sort_by_key(|version| version.parsed_version);
        Ok(result)
    }

    pub async fn install_java(
        &self,
        major_version: u32,
    ) -> Result<theseus::data::JavaVersion, BackendError> {
        self.wait_ready().await?;
        let path = theseus::jre::auto_install_java(major_version)
            .await
            .map_err(BackendError::Operation)?;
        let version = theseus::jre::check_jre(path)
            .await
            .map_err(BackendError::Operation)?;
        theseus::jre::set_java_version(version.clone())
            .await
            .map_err(BackendError::Operation)?;
        Ok(version)
    }

    pub async fn register_java(
        &self,
        path: std::path::PathBuf,
    ) -> Result<theseus::data::JavaVersion, BackendError> {
        self.wait_ready().await?;
        let version = theseus::jre::check_jre(path)
            .await
            .map_err(BackendError::Operation)?;
        theseus::jre::set_java_version(version.clone())
            .await
            .map_err(BackendError::Operation)?;
        Ok(version)
    }

    pub async fn set_instance_launch_settings(
        &self,
        instance_id: &str,
        java_path: Option<String>,
        extra_launch_args: Option<Vec<String>>,
        memory_mb: Option<u32>,
        force_fullscreen: Option<bool>,
        game_resolution: Option<(u16, u16)>,
        custom_env_vars: Option<Vec<(String, String)>>,
        hooks: theseus::data::Hooks,
    ) -> Result<theseus::data::InstanceMetadata, BackendError> {
        self.wait_ready().await?;
        if let Some(path) = java_path.as_ref() {
            theseus::jre::check_jre(std::path::PathBuf::from(path))
                .await
                .map_err(BackendError::Operation)?;
        }
        theseus::instance::edit(
            instance_id,
            theseus::data::EditInstance {
                launch_overrides: Some(
                    theseus::data::InstanceLaunchOverridesPatch {
                        java_path: Some(java_path),
                        extra_launch_args: Some(extra_launch_args),
                        memory: Some(memory_mb.map(|maximum| {
                            theseus::data::MemorySettings { maximum }
                        })),
                        force_fullscreen: Some(force_fullscreen),
                        game_resolution: Some(game_resolution.map(
                            |(width, height)| {
                                theseus::data::WindowSize(width, height)
                            },
                        )),
                        custom_env_vars: Some(custom_env_vars),
                        hooks: Some(hooks),
                        ..Default::default()
                    },
                ),
                ..Default::default()
            },
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn maximum_memory_mb(&self) -> Result<u32, BackendError> {
        self.wait_ready().await?;
        let kib = theseus::jre::get_max_memory()
            .await
            .map_err(BackendError::Operation)?;
        Ok((kib / 1024).min(u32::MAX as u64) as u32)
    }

    pub async fn remove_instance(&self, id: &str) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::remove(id)
            .await
            .map_err(BackendError::Operation)
    }

    /// Starts the actual install job; monitor its status and errors separately.
    pub async fn create_instance(
        &self,
        request: CreateInstance,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::create_instance(
            request.name,
            request.game_version,
            request.loader,
            request.loader_version,
            request.icon_path,
            request.icon_config,
            request.link,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn repair_instance(
        &self,
        instance_id: &str,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        let metadata = theseus::instance::get(instance_id)
            .await
            .map_err(BackendError::Operation)?
            .ok_or_else(|| {
                BackendError::Operation(theseus::Error::msg(
                    "instance no longer exists",
                ))
            })?;
        let pack_location =
            repair_pack_location(&metadata.link, &metadata.instance.name);
        match pack_location {
            Some(location) => {
                theseus::install::install_pack_to_existing_instance(
                    instance_id.to_owned(),
                    location,
                    None,
                )
                .await
                .map_err(BackendError::Operation)
            }
            None => theseus::install::install_existing_instance(
                instance_id.to_owned(),
                false,
            )
            .await
            .map_err(BackendError::Operation),
        }
    }

    pub async fn importable_instances(
        &self,
        launcher: theseus::pack::import::ImportLauncherType,
        base_path: std::path::PathBuf,
    ) -> Result<Vec<String>, BackendError> {
        self.wait_ready().await?;
        theseus::pack::import::get_importable_instances(launcher, base_path)
            .await
            .map_err(BackendError::Operation)
    }

    /// Returns the conventional install directory for a supported third-party launcher.
    pub fn default_launcher_path(
        launcher: theseus::pack::import::ImportLauncherType,
    ) -> Option<std::path::PathBuf> {
        theseus::pack::import::get_default_launcher_path(launcher)
    }

    /// Starts an import job; the caller must select the source explicitly.
    pub async fn import_instance(
        &self,
        launcher: theseus::pack::import::ImportLauncherType,
        base_path: std::path::PathBuf,
        instance_folder: String,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        if !valid_import_folder(&instance_folder) {
            return Err(BackendError::Operation(theseus::Error::msg(
                "import instance folder must be a single directory name",
            )));
        }
        theseus::install::import_instance(launcher, base_path, instance_folder)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn import_pack_file(
        &self,
        path: std::path::PathBuf,
        post_install_edit: Option<theseus::install::InstallPostInstallEdit>,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::create_modpack_instance(
            theseus::pack::install_from::CreatePackLocation::FromFile { path },
            post_install_edit,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn minecraft_skins(
        &self,
    ) -> Result<Vec<theseus::minecraft_skins::Skin>, BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_skins::get_available_skins()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn minecraft_skin_texture(
        &self,
        skin: &theseus::minecraft_skins::Skin,
    ) -> Result<Vec<u8>, BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_skins::normalize_skin_texture(
            &theseus::minecraft_skins::UrlOrBlob::Url((*skin.texture).clone()),
        )
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(BackendError::Operation)
    }

    pub async fn equip_minecraft_skin(
        &self,
        skin: theseus::minecraft_skins::Skin,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_skins::equip_skin(skin)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn add_and_equip_minecraft_skin(
        &self,
        texture: Vec<u8>,
        variant: theseus::minecraft_skins::MinecraftSkinVariant,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_skins::add_and_equip_custom_skin(
            bytes::Bytes::from(texture),
            variant,
            None,
        )
        .await
        .map(|_| ())
        .map_err(BackendError::Operation)
    }

    pub async fn remove_custom_minecraft_skin(
        &self,
        skin: theseus::minecraft_skins::Skin,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::minecraft_skins::remove_custom_skin(skin)
            .await
            .map_err(BackendError::Operation)
    }

    /// Creates a new instance from a Modrinth modpack version.
    pub async fn install_modrinth_pack_version(
        &self,
        project_id: String,
        version_id: String,
        title: String,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::create_modpack_instance(
            theseus::pack::install_from::CreatePackLocation::FromVersionId {
                project_id,
                version_id,
                title,
                icon_url: None,
            },
            None,
        )
        .await
        .map_err(BackendError::Operation)
    }

    pub async fn duplicate_instance(
        &self,
        source_instance_id: String,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::duplicate_instance(source_instance_id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn list_install_jobs(
        &self,
        include_finished: bool,
    ) -> Result<Vec<theseus::install::InstallJobSnapshot>, BackendError> {
        self.wait_ready().await?;
        theseus::install::list_jobs(include_finished)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn get_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::get_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn pause_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::pause_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn resume_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::resume_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn cancel_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::cancel_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn retry_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<theseus::install::InstallJobSnapshot, BackendError> {
        self.wait_ready().await?;
        theseus::install::retry_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn dismiss_install_job(
        &self,
        id: uuid::Uuid,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::install::dismiss_job(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn install_job_support_details(
        &self,
        id: uuid::Uuid,
    ) -> Result<String, BackendError> {
        self.wait_ready().await?;
        theseus::install::job_support_details(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn list_groups(
        &self,
    ) -> Result<Vec<theseus::instance::InstanceGroup>, BackendError> {
        self.wait_ready().await?;
        theseus::instance::list_groups()
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn create_group(
        &self,
        name: String,
    ) -> Result<theseus::instance::InstanceGroup, BackendError> {
        self.wait_ready().await?;
        theseus::instance::create_group(name)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn rename_group(
        &self,
        id: String,
        name: String,
    ) -> Result<theseus::instance::InstanceGroup, BackendError> {
        self.wait_ready().await?;
        theseus::instance::rename_group(id, name)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn delete_group(&self, id: String) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::delete_group(id)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_group_order(
        &self,
        group_ids: Vec<String>,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::set_group_order(group_ids)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn set_group_memberships(
        &self,
        updates: Vec<theseus::instance::InstanceGroupMembershipUpdate>,
    ) -> Result<(), BackendError> {
        self.wait_ready().await?;
        theseus::instance::set_group_memberships(updates)
            .await
            .map_err(BackendError::Operation)
    }

    pub async fn launch_instance(
        &self,
        instance_id: &str,
        quick_play: theseus::instance::QuickPlayType,
    ) -> Result<theseus::data::ProcessMetadata, BackendError> {
        self.wait_ready().await?;
        theseus::instance::run(instance_id, quick_play)
            .await
            .map_err(BackendError::Operation)
    }
}

fn native_ui_preferences_path() -> Result<std::path::PathBuf, BackendError> {
    let app_identifier = APP_IDENTIFIER.get().ok_or_else(|| {
        BackendError::Configuration(
            "native backend has no application identifier".into(),
        )
    })?;
    let settings_dir = std::env::var_os("THESEUS_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_dir().map(|path| path.join(app_identifier)))
        .ok_or_else(|| {
            BackendError::Configuration(
                "could not locate the application settings directory".into(),
            )
        })?;
    Ok(settings_dir.join("native-ui-preferences.json"))
}

fn valid_app_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.split('.').all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                })
        })
}

fn valid_import_folder(folder: &str) -> bool {
    !folder.is_empty()
        && folder != "."
        && folder != ".."
        && !folder
            .chars()
            .any(|ch| matches!(ch, '/' | '\\' | ':') || ch.is_control())
}

fn valid_world_component(world: &str) -> bool {
    !world.is_empty()
        && world != "."
        && world != ".."
        && !world
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
}

fn valid_log_filename(filename: &str) -> bool {
    !filename.is_empty()
        && filename != "."
        && filename != ".."
        && !filename
            .chars()
            .any(|ch| matches!(ch, '/' | '\\' | ':') || ch.is_control())
}

fn repair_pack_location(
    link: &theseus::data::InstanceLink,
    title: &str,
) -> Option<theseus::pack::install_from::CreatePackLocation> {
    match link {
        theseus::data::InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        } => Some(
            theseus::pack::install_from::CreatePackLocation::FromVersionId {
                project_id: project_id.clone(),
                version_id: version_id.clone(),
                title: title.to_owned(),
                icon_url: None,
            },
        ),
        theseus::data::InstanceLink::ServerProjectModpack {
            content_project_id,
            content_version_id,
            ..
        } => Some(
            theseus::pack::install_from::CreatePackLocation::FromVersionId {
                project_id: content_project_id.clone(),
                version_id: content_version_id.clone(),
                title: title.to_owned(),
                icon_url: None,
            },
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mclogs_share_links_require_success_and_trusted_https_host() {
        let valid = serde_json::json!({
            "success": true,
            "url": "https://mclo.gs/abc123"
        });
        assert_eq!(mclogs_share_url(&valid).unwrap(), "https://mclo.gs/abc123");
        for response in [
            serde_json::json!({"success": true, "url": "http://mclo.gs/abc123"}),
            serde_json::json!({"success": true, "url": "https://example.com/abc123"}),
            serde_json::json!({"success": false, "error": "rejected"}),
            serde_json::json!({"success": true}),
        ] {
            assert!(mclogs_share_url(&response).is_err());
        }
    }

    #[test]
    fn native_color_theme_persistence_accepts_all_colorways_and_clamps_invalid_values()
     {
        for theme in 0..=8 {
            assert_eq!(clamp_native_color_theme(theme), theme);
        }
        assert_eq!(clamp_native_color_theme(u8::MAX), 8);
    }

    #[test]
    fn preferred_dark_theme_defaults_and_accepts_only_dark_choices() {
        assert_eq!(
            preferred_dark_theme_name(preferred_dark_theme_from_preferences(
                &serde_json::json!({})
            )),
            "dark"
        );
        assert_eq!(
            preferred_dark_theme_name(preferred_dark_theme_from_preferences(
                &serde_json::json!({"preferred_dark_theme":"oled"})
            )),
            "oled"
        );
        assert_eq!(
            preferred_dark_theme_name(preferred_dark_theme_from_preferences(
                &serde_json::json!({"preferred_dark_theme":"retro"})
            )),
            "dark"
        );
        assert_eq!(
            preferred_dark_theme_name(preferred_dark_theme_from_preferences(
                &serde_json::json!({"preferred_dark_theme":"light"})
            )),
            "dark"
        );
        use theseus::data::Theme;
        assert_eq!(preferred_dark_theme_name(Theme::Light), "dark");
        assert_eq!(preferred_dark_theme_name(Theme::Oled), "oled");
        assert_eq!(preferred_dark_theme_name(Theme::Retro), "dark");
    }

    #[test]
    fn native_theme_sync_setting_defaults_off_and_reads_only_boolean_values() {
        assert!(!native_theme_sync_enabled(&serde_json::json!({})));
        assert!(native_theme_sync_enabled(
            &serde_json::json!({"sync_theme_across_devices": true})
        ));
        assert!(!native_theme_sync_enabled(
            &serde_json::json!({"sync_theme_across_devices": "true"})
        ));
    }

    #[test]
    fn project_follow_url_encodes_the_project_identifier_as_one_path_segment() {
        let url = project_follow_url("abc/def").unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.modrinth.com/v3/project/abc%2Fdef/follow"
        );
    }

    #[test]
    fn collection_url_encodes_the_collection_identifier_as_one_path_segment() {
        let url = collection_url("abc/def").unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.modrinth.com/v3/collection/abc%2Fdef"
        );
    }

    #[test]
    fn hosting_backup_url_encodes_each_resource_id_as_one_path_segment() {
        let url = hosting_backup_url(
            "server/a",
            "world/b",
            &["backups-queue", "backup/c", "restore"],
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://archon.modrinth.com/v1/servers/server%2Fa/worlds/world%2Fb/backups-queue/backup%2Fc/restore"
        );
        let undelete = hosting_backup_url(
            "server/a",
            "world/b",
            &["backups-queue", "backup/c", "undelete"],
        )
        .unwrap();
        assert_eq!(
            undelete.as_str(),
            "https://archon.modrinth.com/v1/servers/server%2Fa/worlds/world%2Fb/backups-queue/backup%2Fc/undelete"
        );
    }

    #[test]
    fn hosting_node_backup_url_strips_filesystem_path_and_encodes_ids_and_token()
     {
        let authorize = hosting_node_backup_url(
            "node.example/modrinth/v0/fs",
            "world/a",
            "backup/b",
            true,
            None,
        )
        .unwrap();
        assert_eq!(
            authorize.as_str(),
            "https://node.example/v1/worlds/world%2Fa/backups/backup%2Fb/download/authorize?format=zip"
        );
        let download = hosting_node_backup_url(
            "https://node.example/modrinth/v0/fs",
            "world/a",
            "backup/b",
            false,
            Some("token&other=value"),
        )
        .unwrap();
        assert_eq!(
            download.as_str(),
            "https://node.example/v1/worlds/world%2Fa/backups/backup%2Fb/download?token=token%26other%3Dvalue"
        );
    }

    #[test]
    fn hosting_node_backup_url_rejects_insecure_or_credentialed_urls() {
        assert!(
            hosting_node_backup_url(
                "http://node.example",
                "world",
                "backup",
                true,
                None,
            )
            .is_err()
        );
        assert!(
            hosting_node_backup_url(
                "https://user:password@node.example",
                "world",
                "backup",
                true,
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn content_service_collection_reads_project_ids() {
        let collection: ContentServiceCollection =
            serde_json::from_value(serde_json::json!({
                "id": "collection-id",
                "name": "Favorites",
                "projects": ["project-a", "project-b"]
            }))
            .unwrap();
        assert_eq!(collection.id, "collection-id");
        assert_eq!(collection.name, "Favorites");
        assert_eq!(collection.projects, ["project-a", "project-b"]);
    }

    #[test]
    fn hosting_access_permissions_map_to_supported_roles() {
        let member = |permissions| HostingServerAccessUser {
            user: HostingServerAccessProfile {
                id: "user-id".into(),
                username: "Player".into(),
                avatar_url: None,
            },
            added_on: None,
            last_invite_sent: None,
            permissions,
        };
        assert!(matches!(
            member(serde_json::json!("BASE_READ")).role(),
            HostingServerAccessRole::Viewer
        ));
        assert!(matches!(
            member(serde_json::json!("FILES_WRITE|BASE_READ")).role(),
            HostingServerAccessRole::Editor
        ));
        assert!(matches!(
            member(serde_json::json!(1u64 << 60)).role(),
            HostingServerAccessRole::Editor
        ));
        assert!(matches!(
            member(serde_json::json!("SERVER_ADMIN")).role(),
            HostingServerAccessRole::Owner
        ));
    }

    #[test]
    fn hosting_reinvite_response_exposes_cooldown() {
        let sent: HostingServerReinviteResponse =
            serde_json::from_str(r#"{"sent":true,"cooldown_seconds":null}"#)
                .unwrap();
        assert!(sent.sent);
        assert_eq!(sent.cooldown_seconds, None);

        let cooling_down: HostingServerReinviteResponse =
            serde_json::from_str(r#"{"sent":false,"cooldown_seconds":42}"#)
                .unwrap();
        assert!(!cooling_down.sent);
        assert_eq!(cooling_down.cooldown_seconds, Some(42));
    }

    #[test]
    fn hosting_backups_deserialize_queue_entries_without_exposing_other_fields()
    {
        let mut backups: HostingBackups =
            serde_json::from_value(serde_json::json!({
                "backups": [{
                    "id": "backup-1",
                    "name": "Before update",
                    "created_at": "2026-11-21T10:00:00Z",
                    "status": "done",
                    "automated": false,
                    "locked": false,
                    "history": [{
                        "operation_type": "create",
                        "user_info": {
                            "id": "user-1",
                            "username": "Frank",
                            "avatar_url": "https://cdn.example.test/avatar.png"
                        }
                    }]
                }],
                "deleted_backups": [{
                    "id": "deleted-1",
                    "name": "Retained snapshot",
                    "created_at": "2026-11-20T10:00:00Z",
                    "status": "done",
                    "automated": true,
                    "created_by": {
                        "id": "support",
                        "username": "Support",
                        "avatar_url": null
                    },
                    "scheduled_delete": "2026-12-01T00:00:00Z",
                    "undelete_deadline": "2026-11-28T00:00:00Z",
                    "deleted_suspension": true,
                    "can_undelete": true,
                    "permanently_deleted": false,
                    "repository_id": 42
                }],
                "active_operations": [],
                "options": {
                    "split_automatic": 3,
                    "split_manual": 5,
                    "limit": 8,
                    "interval": 86400
                },
                "manual_count": 1,
                "auto_count": 0,
                "next_auto_backup_at": null
            }))
            .unwrap();
        backups.world_id = "world-1".into();
        assert_eq!(backups.world_id, "world-1");
        assert_eq!(backups.manual_count, 1);
        assert_eq!(backups.auto_count, 0);
        assert_eq!(backups.options.split_manual, 5);
        assert_eq!(backups.options.split_automatic, 3);
        assert_eq!(backups.options.interval, 86400);
        assert_eq!(backups.backups[0].id, "backup-1");
        assert_eq!(backups.backups[0].status, "done");
        let creator = backups.backups[0].history[0].user_info.as_ref().unwrap();
        assert_eq!(creator.username, "Frank");
        assert_eq!(
            creator.avatar_url.as_deref(),
            Some("https://cdn.example.test/avatar.png")
        );
        assert_eq!(backups.deleted_backups[0].id, "deleted-1");
        assert_eq!(
            backups.deleted_backups[0].created_by.as_ref().unwrap().id,
            "support"
        );
        assert!(backups.deleted_backups[0].can_undelete);
        assert!(backups.deleted_backups[0].deleted_suspension);
    }

    #[test]
    fn hosting_backup_options_omit_interval_when_auto_backups_are_disabled() {
        let disabled = serde_json::to_value(HostingBackupOptionsPatch {
            split_automatic: 0,
            split_manual: 8,
            interval: None,
        })
        .unwrap();
        assert_eq!(disabled["split_automatic"], 0);
        assert_eq!(disabled["split_manual"], 8);
        assert!(disabled.get("interval").is_none());

        let enabled = serde_json::to_value(HostingBackupOptionsPatch {
            split_automatic: 3,
            split_manual: 5,
            interval: Some(24),
        })
        .unwrap();
        assert_eq!(enabled["interval"], 24);
    }

    #[tokio::test]
    async fn event_forwarder_preserves_ordered_bursts() {
        let (sender, source) = broadcast::channel(2048);
        let mut events = forward_events(source).await;
        let count = 1536;
        for index in 0..count {
            let event = if index % 2 == 0 {
                AppEvent::MinimizeWindow
            } else {
                AppEvent::RestoreAndFocusWindow
            };
            assert!(sender.send(event).is_ok());
        }
        for index in 0..count {
            let expected_minimize = index % 2 == 0;
            let received = events.recv().await;
            assert!(matches!(
                (expected_minimize, received),
                (true, Some(NativeEvent::Event(AppEvent::MinimizeWindow)))
                    | (
                        false,
                        Some(NativeEvent::Event(
                            AppEvent::RestoreAndFocusWindow
                        ))
                    )
            ));
        }
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn application_identifier_cannot_escape_its_data_directory() {
        assert!(valid_app_identifier("org.example.launcher-1"));
        for invalid in [
            "",
            ".",
            "..",
            "a..b",
            "../outside",
            "a\\b",
            "C:folder",
            " name",
        ] {
            assert!(!valid_app_identifier(invalid), "accepted {invalid:?}");
        }
    }

    #[test]
    fn import_folder_must_be_one_directory_name() {
        assert!(valid_import_folder("My instance 1.20"));
        assert!(valid_import_folder("世界"));
        for invalid in [
            "",
            ".",
            "..",
            "../outside",
            "a/b",
            "a\\b",
            "C:folder",
            "a\n",
        ] {
            assert!(!valid_import_folder(invalid), "accepted {invalid:?}");
        }
    }

    #[test]
    fn repair_preserves_managed_modpack_source_and_uses_instance_repair_otherwise()
     {
        use theseus::data::InstanceLink;
        use theseus::pack::install_from::CreatePackLocation;

        assert!(matches!(
            repair_pack_location(
                &InstanceLink::ModrinthModpack {
                    project_id: "project".into(),
                    version_id: "version".into(),
                },
                "My pack",
            ),
            Some(CreatePackLocation::FromVersionId {
                project_id,
                version_id,
                title,
                icon_url: None,
            }) if project_id == "project" && version_id == "version" && title == "My pack"
        ));
        assert!(matches!(
            repair_pack_location(
                &InstanceLink::ServerProjectModpack {
                    server_project_id: "server".into(),
                    content_project_id: "content-project".into(),
                    content_version_id: "content-version".into(),
                },
                "Server pack",
            ),
            Some(CreatePackLocation::FromVersionId {
                project_id,
                version_id,
                title,
                icon_url: None,
            }) if project_id == "content-project" && version_id == "content-version" && title == "Server pack"
        ));
        assert!(
            repair_pack_location(&InstanceLink::Unmanaged, "Custom").is_none()
        );
    }

    #[test]
    fn log_filename_must_not_escape_its_log_directory() {
        assert!(valid_log_filename("latest.log"));
        assert!(valid_log_filename("crash-世界.txt"));
        for invalid in [
            "",
            ".",
            "..",
            "../outside.log",
            "a\\b.log",
            "C:secret.log",
            "bad\nname.log",
        ] {
            assert!(!valid_log_filename(invalid), "accepted {invalid:?}");
        }
    }

    #[tokio::test]
    async fn readiness_waits_for_initialization_and_reports_failures() {
        let (sender, startup) = watch::channel(StartupStatus::Starting);
        let backend = NativeBackend { startup };
        let waiter = tokio::spawn(async move { backend.wait_ready().await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        sender.send_replace(StartupStatus::Ready);
        assert!(waiter.await.unwrap().is_ok());

        let (sender, startup) = watch::channel(StartupStatus::Starting);
        let backend = NativeBackend { startup };
        sender
            .send_replace(StartupStatus::Failed("database error".to_string()));
        assert!(matches!(
            backend.wait_ready().await,
            Err(BackendError::Startup(message)) if message == "database error"
        ));
    }

    #[tokio::test]
    async fn readiness_reports_an_abandoned_initialization() {
        let (sender, startup) = watch::channel(StartupStatus::Starting);
        drop(sender);
        let backend = NativeBackend { startup };
        assert!(matches!(
            backend.wait_ready().await,
            Err(BackendError::Startup(_))
        ));
    }

    #[tokio::test]
    async fn discovery_operations_propagate_startup_failure() {
        let (_sender, startup) = watch::channel(StartupStatus::Failed(
            "initialization failed".to_string(),
        ));
        let backend = NativeBackend { startup };
        assert!(matches!(
            backend.search_projects("?query=test", None).await,
            Err(BackendError::Startup(_))
        ));
        assert!(matches!(
            backend.project("test", None).await,
            Err(BackendError::Startup(_))
        ));
        assert!(matches!(
            backend.project_versions("test", None).await,
            Err(BackendError::Startup(_))
        ));
        assert!(matches!(
            backend.instance_worlds("test").await,
            Err(BackendError::Startup(_))
        ));
        assert!(matches!(
            backend
                .set_world_display_status(
                    "test",
                    theseus::worlds::WorldType::Singleplayer,
                    "world",
                    theseus::worlds::DisplayStatus::Favorite,
                )
                .await,
            Err(BackendError::Startup(_))
        ));
    }
}

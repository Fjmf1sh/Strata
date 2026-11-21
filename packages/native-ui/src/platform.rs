//! Operating-system adapters used by the native launcher UI.

#[cfg(windows)]
pub(super) fn clipboard_text() -> Result<String, String> {
    clipboard_win::get_clipboard_string().map_err(|error| error.to_string())
}

#[cfg(windows)]
pub(super) fn set_clipboard_text(text: &str) -> Result<(), String> {
    clipboard_win::set_clipboard_string(text).map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
pub(super) fn clipboard_text() -> Result<String, String> {
    let output = std::process::Command::new("pbpaste")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("pbpaste could not read the clipboard".into());
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
pub(super) fn set_clipboard_text(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("pbcopy stdin unavailable")?
        .write_all(text.as_bytes())
        .map_err(|error| error.to_string())?;
    let status = child.wait().map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("pbcopy could not write the clipboard".into())
    }
}

#[cfg(target_os = "linux")]
pub(super) fn clipboard_text() -> Result<String, String> {
    for (program, args) in [
        ("wl-paste", vec!["--no-newline"]),
        ("xclip", vec!["-selection", "clipboard", "-o"]),
        ("xsel", vec!["--clipboard", "--output"]),
    ] {
        if let Ok(output) =
            std::process::Command::new(program).args(args).output()
        {
            if output.status.success() {
                return String::from_utf8(output.stdout)
                    .map_err(|error| error.to_string());
            }
        }
    }
    Err(
        "install wl-clipboard, xclip, or xsel to paste the authorization code"
            .into(),
    )
}

#[cfg(target_os = "linux")]
pub(super) fn set_clipboard_text(text: &str) -> Result<(), String> {
    use std::io::Write;
    for (program, args) in [
        ("wl-copy", vec![]),
        ("xclip", vec!["-selection", "clipboard"]),
        ("xsel", vec!["--clipboard", "--input"]),
    ] {
        let Ok(mut child) = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        else {
            continue;
        };
        let Some(mut stdin) = child.stdin.take() else {
            continue;
        };
        if stdin.write_all(text.as_bytes()).is_err() {
            continue;
        }
        drop(stdin);
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(None) => return Ok(()),
            _ => continue,
        }
    }
    Err("install wl-clipboard, xclip, or xsel to copy the invite link".into())
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(super) fn clipboard_text() -> Result<String, String> {
    Err("clipboard access is not supported on this platform".into())
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(super) fn set_clipboard_text(_text: &str) -> Result<(), String> {
    Err("clipboard access is not supported on this platform".into())
}

#[cfg(any(windows, test))]
pub(super) fn screenshot_clipboard_bitmap(
    image: image::DynamicImage,
) -> Result<Vec<u8>, String> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image.into_rgb8())
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .map_err(|error| error.to_string())?;
    Ok(bytes.into_inner())
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn load_screenshot_for_clipboard(
    path: &std::path::Path,
) -> Result<image::DynamicImage, String> {
    let mut reader = image::ImageReader::open(path)
        .map_err(|error| error.to_string())?
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|error| error.to_string())
}

#[cfg(any(windows, target_os = "linux", target_os = "macos", test))]
pub(super) fn screenshot_clipboard_png(
    image: &image::DynamicImage,
) -> Result<Vec<u8>, String> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(bytes.into_inner())
}

#[cfg(windows)]
pub(super) fn copy_screenshot_image(
    path: &std::path::Path,
) -> Result<(), String> {
    let image = load_screenshot_for_clipboard(path)?;
    let png = screenshot_clipboard_png(&image)?;
    let bitmap = screenshot_clipboard_bitmap(image)?;
    let format = clipboard_win::raw::register_format("PNG")
        .ok_or("Could not register PNG clipboard format")?;
    let _clipboard = clipboard_win::Clipboard::new_attempts(10)
        .map_err(|error| error.to_string())?;
    clipboard_win::raw::empty().map_err(|error| error.to_string())?;
    clipboard_win::raw::set_without_clear(format.get(), &png)
        .map_err(|error| error.to_string())?;
    clipboard_win::raw::set_bitmap(&bitmap).map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
pub(super) fn copy_screenshot_image(
    path: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write;
    let bytes =
        screenshot_clipboard_png(&load_screenshot_for_clipboard(path)?)?;
    for (command, args) in [
        ("wl-copy", &(["--type", "image/png"])[..]),
        (
            "xclip",
            &["-selection", "clipboard", "-t", "image/png", "-i"][..],
        ),
    ] {
        if let Ok(mut child) = std::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            let written = child
                .stdin
                .take()
                .ok_or("Could not open clipboard input")?
                .write_all(&bytes);
            if written.is_ok() {
                match child.try_wait() {
                    Ok(Some(status)) if status.success() => return Ok(()),
                    Ok(None) => return Ok(()),
                    _ => continue,
                }
            }
        }
    }
    Err("Could not copy image; install wl-copy or xclip".into())
}

#[cfg(target_os = "macos")]
pub(super) fn copy_screenshot_image(
    path: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write;

    struct TempImage(std::path::PathBuf);
    impl Drop for TempImage {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    let bytes =
        screenshot_clipboard_png(&load_screenshot_for_clipboard(path)?)?;
    let path = std::env::temp_dir()
        .join(format!("northstar-copy-{}.png", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    let temp = TempImage(path);
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    drop(file);
    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(
            "on run argv\nset the clipboard to (read (POSIX file (item 1 of argv)) as «class PNGf»)\nend run",
        )
        .arg(&temp.0)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not copy image to clipboard".into())
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(super) fn copy_screenshot_image(
    _path: &std::path::Path,
) -> Result<(), String> {
    Err("Image clipboard access is not supported on this platform".into())
}

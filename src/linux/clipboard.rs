//! Text and PNG through X11 selections, except on KDE Wayland and Hyprland: neither reliably
//! bridges a background XWayland owner's selection to native apps. There we use wl-copy's
//! data-control protocol (bundled in the AppImage). Both backends keep an owner alive to serve
//! the data.

use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow};

static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn with<T>(f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, arboard::Error>) -> Result<T> {
    let mut slot = CLIPBOARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if slot.is_none() {
        *slot = Some(arboard::Clipboard::new().map_err(|e| anyhow!("{e}"))?);
    }
    let clipboard = slot.as_mut().context("No clipboard")?;
    f(clipboard).map_err(|e| anyhow!("Clipboard: {e}"))
}

/// Desktops whose XWayland selections don't reach native Wayland apps.
fn native_wayland_clipboard() -> bool {
    (super::kde() || super::hyprland()) && super::wayland_session()
}

pub struct TextSnapshot {
    mime: String,
    bytes: Vec<u8>,
}

fn plain_text_mime(types: &str) -> Result<&str> {
    let offered: Vec<_> = types.lines().collect();
    anyhow::ensure!(
        !offered.is_empty()
            && offered.iter().all(|mime| matches!(
                *mime,
                "text/plain" | "text/plain;charset=utf-8" | "UTF8_STRING" | "STRING" | "TEXT"
            )),
        "Only plain-text clipboards can be restored by draft handoff"
    );
    offered.iter().copied()
        .find(|mime| *mime == "text/plain;charset=utf-8")
        .or_else(|| offered.iter().copied().find(|mime| *mime == "text/plain"))
        .context("Clipboard has no plain-text MIME type")
}

pub fn snapshot_text_wayland() -> Result<TextSnapshot> {
    anyhow::ensure!(native_wayland_clipboard(), "Draft clipboard restoration requires KDE or Hyprland Wayland");
    let types = read_wayland(&["--list-types"], 8192)?;
    let types = std::str::from_utf8(&types).context("Invalid clipboard types")?;
    let mime = plain_text_mime(types)?.to_owned();
    let bytes = read_wayland(&["--no-newline", "--type", &mime], 4 * 1024 * 1024)?;
    Ok(TextSnapshot { mime, bytes })
}

pub fn restore_text_wayland(snapshot: TextSnapshot, offered: Option<(&str, &[u8])>) -> Result<()> {
    let Some((mime, expected)) = offered else { return Ok(()); };
    anyhow::ensure!(
        still_ours(mime, expected)?,
        "Clipboard changed during handoff; leaving the newer content untouched"
    );
    copy_wayland(&snapshot.mime, &snapshot.bytes)
}

pub fn still_ours(mime: &str, expected: &[u8]) -> Result<bool> {
    let offered = read_wayland(&["--list-types"], 8192)?;
    let offered = std::str::from_utf8(&offered).context("Invalid clipboard types")?;
    if !offered.lines().any(|item| item == mime) {
        return Ok(false);
    }
    Ok(read_wayland(&["--no-newline", "--type", mime], 4 * 1024 * 1024)? == expected)
}

fn read_wayland(args: &[&str], limit: usize) -> Result<Vec<u8>> {
    let bundled = std::env::current_exe().ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("wl-paste")))
        .filter(|path| path.is_file());
    let mut child = Command::new("timeout")
        .arg("3s")
        .arg(bundled.unwrap_or_else(|| "wl-paste".into()))
        .args(args)
        .env("WAYLAND_DISPLAY", super::wayland_display().context("No Wayland display")?)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn().context("Native Wayland clipboard needs wl-paste and timeout")?;
    let mut bytes = Vec::new();
    let read = child.stdout.take().context("No clipboard output")?
        .take(limit as u64 + 1).read_to_end(&mut bytes);
    if bytes.len() > limit || read.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().context("Waiting for wl-paste")?;
    read.context("Reading clipboard")?;
    anyhow::ensure!(bytes.len() <= limit, "Clipboard too large to restore safely");
    anyhow::ensure!(status.success(), "Could not read clipboard (wl-paste: {status})");
    Ok(bytes)
}

pub fn set_text(text: &str) -> Result<()> {
    if native_wayland_clipboard() {
        return copy_wayland("text/plain;charset=utf-8", text.as_bytes());
    }
    with(|c| c.set_text(text.to_owned()))
}

pub fn set_png(png_bytes: &[u8]) -> Result<()> {
    if native_wayland_clipboard() {
        return copy_wayland("image/png", png_bytes);
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().context("Not a PNG")?;
    let mut buffer = vec![0; reader.output_buffer_size().context("PNG too large")?];
    let info = reader.next_frame(&mut buffer)?;
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buffer[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => anyhow::bail!("Unsupported PNG colour type {other:?}"),
    };
    with(|c| {
        c.set_image(arboard::ImageData {
            width: info.width as usize,
            height: info.height as usize,
            bytes: rgba.into(),
        })
    })
}

fn copy_wayland(mime: &str, bytes: &[u8]) -> Result<()> {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("wl-copy")))
        .filter(|path| path.is_file());
    let mut child = Command::new(bundled.unwrap_or_else(|| "wl-copy".into()))
        .args(["--type", mime])
        .env(
            "WAYLAND_DISPLAY",
            super::wayland_display().context("No Wayland display")?,
        )
        .stdin(Stdio::piped())
        // wl-copy forks a clipboard owner after acquiring the selection. It must not keep
        // pipes to Scrawl open; waiting for captured stdout/stderr would never finish.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Native Wayland clipboard needs wl-copy (install wl-clipboard)")?;
    let written = child
        .stdin
        .take()
        .context("No clipboard input")?
        .write_all(bytes);
    let status = child.wait().context("Waiting for wl-copy")?;
    written.context("Writing the Wayland clipboard")?;
    anyhow::ensure!(status.success(), "wl-copy failed: {status}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_policy_accepts_only_plain_text_and_rejects_rich_or_image_clipboards() {
        assert_eq!(plain_text_mime("text/plain;charset=utf-8\ntext/plain\n" ).unwrap(), "text/plain;charset=utf-8");
        assert!(plain_text_mime("text/plain\nimage/png\n").is_err());
        assert!(plain_text_mime("text/html\ntext/plain\n").is_err());
        assert!(plain_text_mime("image/png\n").is_err());
        assert!(plain_text_mime("").is_err());
    }

    #[test]
    fn isolated_wayland_restores_exact_text_after_png_offer() {
        if std::env::var_os("SCRAWL_TEST_WAYLAND_CLIPBOARD").is_none() {
            return;
        }
        let original = b"synthetic original\n";
        copy_wayland("text/plain;charset=utf-8", original).unwrap();
        let snapshot = snapshot_text_wayland().unwrap();
        assert_eq!(snapshot.bytes, original);
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&[255, 0, 0, 255]).unwrap();
        }
        copy_wayland("image/png", &png).unwrap();
        assert!(snapshot_text_wayland().is_err(), "image clipboard is not restorable as text");
        restore_text_wayland(snapshot, Some(("image/png", &png))).unwrap();
        assert_eq!(read_wayland(&["--no-newline", "--type", "text/plain;charset=utf-8"], 1024).unwrap(), original);
        let snapshot = snapshot_text_wayland().unwrap();
        copy_wayland("image/png", &png).unwrap();
        copy_wayland("text/plain;charset=utf-8", b"newer synthetic clipboard").unwrap();
        assert!(restore_text_wayland(snapshot, Some(("image/png", &png))).is_err());
        assert_eq!(
            read_wayland(&["--no-newline", "--type", "text/plain;charset=utf-8"], 1024).unwrap(),
            b"newer synthetic clipboard"
        );
    }
}

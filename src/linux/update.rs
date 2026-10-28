//! AppImage updates. The running inode remains in use; the new image starts on the next launch.
//! Never replace a debug build, a symlink, or somebody else's /opt installation.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const REPO: &str = "simonvanlaak/scrawl";
const MAIN_FEED: &str = "https://raw.githubusercontent.com/simonvanlaak/scrawl/linux-main-feed/main-updates.json";
static CHECK: Mutex<()> = Mutex::new(());

pub fn set_channel_in(path: &Path, channel: &str) -> Result<()> {
    ensure!(matches!(channel, "stable" | "main"), "channel must be stable or main");
    crate::handoff::set_config_value_in(path, "updateChannel", channel.into())
}

fn write_runtime_in(path: &Path, pid: u32, ticks: u64) -> Result<()> {
    let revision = option_env!("GITHUB_SHA").filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()));
    let installed = target()?.and_then(|image| installed_id(&image));
    crate::handoff::set_config_value_in(path, "running", serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"), "revision": revision,
        "pid": pid, "startTicks": ticks,
        "executable": fs::read_link("/proc/self/exe")?,
        "updateId": installed.as_ref().map(|(_, id)| id),
        "updateChannel": installed.as_ref().map(|(channel, _)| channel),
    }))
}

fn process_ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(") ")?.1.split_whitespace().nth(19)?.parse().ok()
}

fn same_executable(current: &Path, recorded: &str) -> bool {
    // Linux appends this suffix to /proc/PID/exe after the AppImage is exchanged.
    // The running process keeps its old version even though its inode moved.
    current.to_str().is_some_and(|path| path.strip_suffix(" (deleted)").unwrap_or(path) == recorded)
}

pub fn record_running() -> Result<()> {
    let pid = std::process::id();
    write_runtime_in(&crate::handoff::default_config_path(), pid,
        process_ticks(pid).context("cannot read Scrawl process start time")?)
}

pub fn status_at(path: &Path) -> Result<serde_json::Value> {
    let config: serde_json::Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(error) => return Err(error.into()),
    };
    ensure!(config.is_object(), "config must be an object");
    let channel = if config["updateChannel"] == "main" { "main" } else { "stable" };
    let recorded = &config["running"];
    let running = match (recorded["pid"].as_u64().and_then(|n| u32::try_from(n).ok()), recorded["startTicks"].as_u64()) {
        (Some(pid), Some(ticks)) if process_ticks(pid) == Some(ticks)
            && fs::read_link(format!("/proc/{pid}/exe")).ok().as_deref()
                .zip(recorded["executable"].as_str())
                .is_some_and(|(current, original)| same_executable(current, original)) => recorded.clone(),
        _ => serde_json::Value::Null,
    };
    let image = target()?;
    let installed = image.as_ref().and_then(|path| installed_id(path));
    let pending_restart = installed.as_ref().is_some_and(|(feed, id)| {
        running.is_object() && (running["updateId"].as_str() != Some(id.as_str()) ||
            running["updateChannel"].as_str() != Some(feed.as_str()))
    });
    Ok(serde_json::json!({
        "channel": channel, "running": running,
        "installed": installed.as_ref().map(|(feed, id)| serde_json::json!({"channel": feed, "id": id})),
        "pendingRestart": pending_restart, "canUpdate": image.is_some(),
    }))
}

pub fn status() -> Result<serde_json::Value> {
    status_at(&crate::handoff::default_config_path())
}

pub fn check_cli() -> Result<&'static str> {
    target()?.context("Updates require an AppImage owned by you under your home directory")?;
    let _guard = CHECK.lock().map_err(|_| anyhow::anyhow!("update check lock poisoned"))?;
    Ok(if check()? {
        "installed; restart Scrawl to use it"
    } else if status()?["pendingRestart"] == true {
        "already installed; restart Scrawl to use it"
    } else {
        "up to date"
    })
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct MainFeed {
    sha: String,
    assets: std::collections::BTreeMap<String, Asset>,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

struct Candidate {
    id: String,
    channel: &'static str,
    url: String,
    sha256: String,
}

fn arch() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("x86_64"),
        "aarch64" => Some("aarch64"),
        _ => None,
    }
}

fn main_candidate(feed: MainFeed, arch: &str) -> Result<Candidate> {
    ensure!(feed.sha.len() == 40 && feed.sha.bytes().all(|b| b.is_ascii_hexdigit()), "invalid main revision");
    let asset = feed.assets.get(arch).context("main has no AppImage for this CPU")?;
    let prefix = "Scrawl-";
    let suffix = format!("-linux-{arch}.AppImage");
    let version = asset.name.strip_prefix(prefix).and_then(|n| n.strip_suffix(&suffix))
        .context("unexpected main AppImage name")?;
    ensure!(!version.is_empty() && version.bytes().all(|b| b.is_ascii_digit() || b == b'.'), "invalid main version");
    validate_hash(&asset.sha256)?;
    Ok(Candidate {
        id: feed.sha.clone(),
        channel: "main",
        url: format!("https://github.com/{REPO}/releases/download/linux-main-{}/{}", feed.sha, asset.name),
        sha256: asset.sha256.clone(),
    })
}

fn validate_hash(hash: &str) -> Result<()> {
    ensure!(hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()), "invalid SHA-256");
    Ok(())
}

fn get(url: &str, out: Option<&Path>) -> Result<Vec<u8>> {
    ensure!(url.starts_with("https://"), "update URL must use HTTPS");
    let mut cmd = Command::new("curl");
    cmd.args(["--fail", "--silent", "--show-error", "--location", "--proto", "=https", "--proto-redir", "=https", "--connect-timeout", "10", "--max-time", "300", "--user-agent", "Scrawl", "--url", url]);
    if let Some(path) = out {
        cmd.args(["--max-filesize", "1073741824"]);
        cmd.arg("--output").arg(path);
    } else {
        cmd.args(["--max-filesize", "262144"]);
    }
    let result = cmd.output().context("curl is required for updates")?;
    ensure!(result.status.success(), "download failed: {}", String::from_utf8_lossy(&result.stderr));
    Ok(result.stdout)
}

fn stable_candidate(arch: &str) -> Result<Candidate> {
    let body = get(&format!("https://api.github.com/repos/{REPO}/releases/latest"), None)?;
    let release: Release = serde_json::from_slice(&body).context("invalid stable release")?;
    ensure!(release.tag_name.starts_with('v') && release.tag_name[1..].bytes().all(|b| b.is_ascii_digit() || b == b'.'), "invalid stable tag");
    let name = format!("Scrawl-{}-linux-{arch}.AppImage", &release.tag_name[1..]);
    let image = release.assets.iter().find(|a| a.name == name).context("no Linux AppImage in the stable release yet")?;
    let checksum = release.assets.iter().find(|a| a.name == format!("{name}.sha256")).context("stable release has no checksum")?;
    let prefix = format!("https://github.com/{REPO}/releases/download/{}/", release.tag_name);
    ensure!(image.browser_download_url == format!("{prefix}{name}") && checksum.browser_download_url == format!("{prefix}{name}.sha256"), "unexpected release asset URL");
    let body = get(&checksum.browser_download_url, None)?;
    let text = std::str::from_utf8(&body)?;
    let (hash, file) = text.trim().split_once(char::is_whitespace).context("invalid release checksum")?;
    ensure!(file.trim() == name, "checksum refers to another file");
    validate_hash(hash)?;
    Ok(Candidate { id: release.tag_name, channel: "stable", url: image.browser_download_url.clone(), sha256: hash.to_owned() })
}

fn installed_id(image: &Path) -> Option<(String, String)> {
    let sidecar = image.with_extension("AppImage.scrawl-update.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(sidecar).ok()?).ok()?;
    Some((value["channel"].as_str()?.to_owned(), value["id"].as_str()?.to_owned()))
}

fn newer(candidate: &Candidate, installed: Option<(String, String)>) -> bool {
    let version = |s: &str| {
        s.trim_start_matches('v').split('.').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>()
    };
    if let Some((channel, id)) = installed {
        if channel == candidate.channel {
            if channel == "stable" {
                return matches!((version(&candidate.id), version(&id)), (Ok(r), Ok(c)) if r > c);
            }
            return id != candidate.id;
        }
        // Switching channels installs the requested channel even when version strings match.
        return true;
    }
    if candidate.channel == "stable" {
        return matches!((version(&candidate.id), version(env!("CARGO_PKG_VERSION"))), (Ok(r), Ok(c)) if r > c);
    }
    // Old AppImages have no revision embedded; opt-in Main replaces them once.
    true
}

fn target() -> Result<Option<PathBuf>> {
    let Some(path) = std::env::var_os("APPIMAGE").map(PathBuf::from) else { return Ok(None) };
    let meta = fs::symlink_metadata(&path)?;
    ensure!(meta.is_file() && !meta.file_type().is_symlink(), "update target must be an ordinary AppImage");
    ensure!(meta.uid() == unsafe { libc::geteuid() }, "update target belongs to another user");
    let home = dirs::home_dir().context("home directory unavailable")?;
    let path = path.canonicalize()?;
    ensure!(path.starts_with(home.canonicalize()?), "refusing to replace an AppImage outside your home");
    Ok(Some(path))
}

fn verify_download(stage: &Path, hash: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut file = File::open(stage)?;
    let mut bytes = [0u8; 128 * 1024];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 { break; }
        hasher.update(&bytes[..n]);
    }
    ensure!(format!("{:x}", hasher.finalize()) == hash, "AppImage checksum mismatch");
    let mut perms = fs::metadata(stage)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(stage, perms)?;
    File::open(stage)?.sync_all()?;
    Ok(())
}

fn exchange(stage: &Path, image: &Path) -> Result<()> {
    let a = std::ffi::CString::new(stage.as_os_str().as_encoded_bytes())?;
    let b = std::ffi::CString::new(image.as_os_str().as_encoded_bytes())?;
    let rc = unsafe { libc::renameat2(libc::AT_FDCWD, a.as_ptr(), libc::AT_FDCWD, b.as_ptr(), libc::RENAME_EXCHANGE) };
    ensure!(rc == 0, "atomic AppImage swap failed: {}", std::io::Error::last_os_error());
    Ok(())
}

fn install(candidate: &Candidate, image: &Path) -> Result<()> {
    let parent = image.parent().context("AppImage has no parent")?;
    let name = image.file_name().context("AppImage has no name")?.to_string_lossy();
    let stage = parent.join(format!(".{name}.download-{}", std::process::id()));
    let backup = parent.join(format!(".{name}.previous"));
    let sidecar = image.with_extension("AppImage.scrawl-update.json");
    let staged_sidecar = parent.join(format!(".{name}.update-{}", std::process::id()));
    // Reserve only our own temporary path; never reuse another process's download.
    let _reserved = OpenOptions::new().write(true).create_new(true).open(&stage)?;
    let mut swapped = false;
    let result = (|| -> Result<()> {
        get(&candidate.url, Some(&stage))?;
        verify_download(&stage, &candidate.sha256)?;
        // Fail closed if the launcher changed while the download was in flight.
        let current = fs::symlink_metadata(image)?;
        ensure!(current.is_file() && !current.file_type().is_symlink() && current.uid() == unsafe { libc::geteuid() }, "installed AppImage changed while downloading");
        let config = crate::handoff::load_config_from(&crate::handoff::default_config_path());
        let selected = if config.update_channel.as_deref() == Some("main") { "main" } else { "stable" };
        ensure!(selected == candidate.channel, "channel changed while downloading");
        let mut f = OpenOptions::new().write(true).create_new(true).open(&staged_sidecar)?;
        f.write_all(serde_json::to_string(&serde_json::json!({"channel": candidate.channel, "id": candidate.id}))?.as_bytes())?;
        f.sync_all()?;
        exchange(&stage, image)?;
        swapped = true;
        // The old AppImage is now at stage; it remains a rollback copy.
        if let Err(error) = fs::rename(&stage, &backup) {
            crate::capture::scrawl_log(format!("Linux update: previous AppImage remains at {}: {error}", stage.display()));
        }
        if let Err(error) = fs::rename(&staged_sidecar, &sidecar) {
            crate::capture::scrawl_log(format!("Linux update: updated AppImage but could not write version marker: {error}"));
        }
        Ok(())
    })();
    // If the swap succeeded, stage no longer exists; never touch the installed image here.
    if !swapped && stage.exists() { let _ = fs::remove_file(&stage); }
    if staged_sidecar.exists() { let _ = fs::remove_file(&staged_sidecar); }
    result
}

fn check() -> Result<bool> {
    let Some(image) = target()? else { return Ok(false) };
    let status = status()?;
    let arch = arch().context("no AppImage for this CPU")?;
    let candidate = if status["channel"] == "main" {
        let feed: MainFeed = serde_json::from_slice(&get(MAIN_FEED, None)?)?;
        main_candidate(feed, arch)?
    } else {
        stable_candidate(arch)?
    };
    if !newer(&candidate, installed_id(&image)) { return Ok(false) }
    install(&candidate, &image)?;
    super::shell::notify("Scrawl updated", "The new version will start the next time you open Scrawl. Your previous AppImage is kept beside it.");
    Ok(true)
}

pub fn check_now() {
    if let Ok(_guard) = CHECK.try_lock()
        && let Err(error) = check()
    {
        crate::capture::scrawl_log(format!("Linux update: {error:#}"));
    }
}

pub fn start() {
    if std::env::var_os("APPIMAGE").is_none() { return }
    std::thread::spawn(|| loop {
        check_now();
        std::thread::sleep(Duration::from_secs(3600));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_feed_rejects_foreign_name_and_digest() {
        let feed = |name: &str, hash: &str| MainFeed {
            sha: "a".repeat(40),
            assets: [("x86_64".into(), Asset { name: name.into(), sha256: hash.into() })].into(),
        };
        let expected = format!("Scrawl-{}-linux-x86_64.AppImage", env!("CARGO_PKG_VERSION"));
        assert!(main_candidate(feed("other.AppImage", &"b".repeat(64)), "x86_64").is_err());
        assert!(main_candidate(feed(&expected, "invalid"), "x86_64").is_err());
        let candidate = main_candidate(feed(&expected, &"b".repeat(64)), "x86_64").unwrap();
        assert!(candidate.url.contains("linux-main-"));
        assert!(main_candidate(feed("Scrawl-0.4.0-linux-x86_64.AppImage", &"b".repeat(64)), "x86_64").is_ok());
        assert!(!newer(&candidate, Some(("main".into(), "a".repeat(40)))));
        assert!(newer(&candidate, Some(("stable".into(), "v0.3.13".into()))));
    }

    #[test]
    fn bad_download_cannot_replace_the_image_and_exchange_preserves_the_previous_one() {
        let dir = std::env::temp_dir().join(format!("scrawl-update-test-{}", crate::make_bundle_id()));
        fs::create_dir(&dir).unwrap();
        let image = dir.join("Scrawl.AppImage");
        let stage = dir.join("download");
        fs::write(&image, b"old image").unwrap();
        fs::write(&stage, b"new image").unwrap();
        assert!(verify_download(&stage, &"0".repeat(64)).is_err());
        assert_eq!(fs::read(&image).unwrap(), b"old image");
        let hash = format!("{:x}", Sha256::digest(b"new image"));
        verify_download(&stage, &hash).unwrap();
        exchange(&stage, &image).unwrap();
        assert_eq!(fs::read(&image).unwrap(), b"new image");
        assert_eq!(fs::read(&stage).unwrap(), b"old image");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stable_never_downgrades_without_a_channel_change() {
        let candidate = Candidate { id: "v0.3.13".into(), channel: "stable", url: String::new(), sha256: String::new() };
        assert!(!newer(&candidate, Some(("stable".into(), "v0.3.14".into()))));
        assert!(!newer(&candidate, Some(("stable".into(), "v0.3.13".into()))));
        assert!(newer(&candidate, Some(("stable".into(), "v0.3.12".into()))));
        assert!(newer(&candidate, Some(("main".into(), "a".repeat(40)))));
    }

    #[test]
    fn channel_command_preserves_other_config_and_rejects_invalid_choices() {
        let dir = std::env::temp_dir().join(crate::make_bundle_id());
        fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, br#"{"ingestToken":"private","autoSend":false}"#).unwrap();
        set_channel_in(&path, "main").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["updateChannel"], "main");
        assert_eq!(value["ingestToken"], "private");
        assert_eq!(value["autoSend"], false);
        let before = fs::read(&path).unwrap();
        assert!(set_channel_in(&path, "edge").is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn config_records_the_running_version_without_touching_preferences() {
        let dir = std::env::temp_dir().join(crate::make_bundle_id());
        fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, br#"{"updateChannel":"main","ingestToken":"private"}"#).unwrap();
        write_runtime_in(&path, 42, 99).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["running"]["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["running"]["pid"], 42);
        assert_eq!(value["running"]["startTicks"], 99);
        assert_eq!(value["updateChannel"], "main");
        assert_eq!(value["ingestToken"], "private");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn status_only_reports_a_live_process_with_the_recorded_start_time() {
        let dir = std::env::temp_dir().join(crate::make_bundle_id());
        fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        let pid = std::process::id();
        let ticks = process_ticks(pid).unwrap();
        write_runtime_in(&path, pid, ticks).unwrap();
        assert_eq!(status_at(&path).unwrap()["running"]["version"], env!("CARGO_PKG_VERSION"));
        write_runtime_in(&path, pid, ticks + 1).unwrap();
        assert!(status_at(&path).unwrap()["running"].is_null());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn exchanged_appimage_still_identifies_its_running_process() {
        assert!(same_executable(Path::new("/home/user/Scrawl.AppImage"), "/home/user/Scrawl.AppImage"));
        assert!(same_executable(Path::new("/home/user/Scrawl.AppImage (deleted)"), "/home/user/Scrawl.AppImage"));
        assert!(!same_executable(Path::new("/home/user/Other.AppImage (deleted)"), "/home/user/Scrawl.AppImage"));
    }
}

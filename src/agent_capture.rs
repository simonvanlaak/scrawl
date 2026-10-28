//! Local, same-user control of the running GUI capture. Never starts or sends a capture.
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Selecting,
    Starting,
    Recording,
    Processing,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct State {
    pub phase: Phase,
    /// Opaque, per-capture identity. Absent outside a pending or active capture.
    pub capture_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Stop,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Stopping,
    Cancelled,
    AlreadyEnded,
    WrongCapture,
    NotReady,
}

pub fn decide(state: &State, action: Action, id: &str) -> Outcome {
    let Some(current) = &state.capture_id else {
        return Outcome::AlreadyEnded;
    };
    if id != current {
        return Outcome::WrongCapture;
    }
    match (&state.phase, action) {
        (Phase::Recording, Action::Stop) => Outcome::Stopping,
        (Phase::Starting | Phase::Recording, Action::Cancel) => Outcome::Cancelled,
        _ => Outcome::NotReady,
    }
}

pub fn new_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn socket_path(create: bool) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join(".config/apparent/agent-control");
    if create {
        std::fs::create_dir_all(&dir)?;
    }
    let meta = std::fs::symlink_metadata(&dir)?;
    ensure!(
        meta.is_dir() && meta.uid() == unsafe { libc::geteuid() },
        "unsafe control directory"
    );
    if create {
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    ensure!(
        std::fs::metadata(&dir)?.permissions().mode() & 0o077 == 0,
        "control directory is not private"
    );
    Ok(dir.join("capture.sock"))
}

fn same_user(stream: &UnixStream) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut _ as *mut _,
                &mut len,
            )
        };
        ensure!(
            rc == 0
                && len as usize == std::mem::size_of::<libc::ucred>()
                && cred.uid == unsafe { libc::geteuid() },
            "control peer is not this user"
        );
    }
    #[cfg(target_os = "macos")]
    {
        let mut uid = 0;
        let mut gid = 0;
        let rc = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
        ensure!(
            rc == 0 && uid == unsafe { libc::geteuid() },
            "control peer is not this user"
        );
    }
    Ok(())
}

pub struct Request {
    pub action: Option<(Action, String)>,
    pub reply: std::sync::mpsc::SyncSender<String>,
}

/// Separate from Linux's shortcut socket: no toggle, no unacknowledged mutation.
pub fn listen() -> Result<flume::Receiver<Request>> {
    let path = socket_path(true)?;
    listen_at(path)
}

fn listen_at(path: PathBuf) -> Result<flume::Receiver<Request>> {
    // Hold this advisory lock for the listener's whole lifetime. Without it, two launches
    // recovering the same stale socket can unlink each other's freshly bound listener.
    let owner_path = path.with_extension("lock");
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(owner_path)?;
    let owner_meta = owner.metadata()?;
    ensure!(
        owner_meta.is_file()
            && owner_meta.uid() == unsafe { libc::geteuid() }
            && owner_meta.mode() & 0o777 == 0o600,
        "unsafe control owner lock"
    );
    ensure!(
        unsafe { libc::flock(owner.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "control socket is already owned by another GUI"
    );
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        ensure!(
            meta.file_type().is_socket()
                && meta.uid() == unsafe { libc::geteuid() }
                && meta.mode() & 0o777 == 0o600,
            "unsafe existing control socket"
        );
        match UnixStream::connect(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                std::fs::remove_file(&path)?
            }
            _ => bail!("control socket is already in use: {}", path.display()),
        }
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let (tx, rx) = flume::bounded(16);
    std::thread::Builder::new()
        .name("apparent-agent-control".into())
        .spawn(move || {
            let _owner = owner;
            while !tx.is_disconnected() {
                let (mut stream, _) = match listener.accept() {
                    Ok(accepted) => accepted,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    Err(_) => break,
                };
                if same_user(&stream).is_err() {
                    continue;
                }
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                let mut line = String::new();
                let response = if BufReader::new(&stream)
                    .take(256)
                    .read_line(&mut line)
                    .is_ok()
                {
                    let action = match line.trim().split_once(' ') {
                        Some(("stop", id)) if valid_id(id) => Some((Action::Stop, id.to_owned())),
                        Some(("cancel", id)) if valid_id(id) => {
                            Some((Action::Cancel, id.to_owned()))
                        }
                        _ => None,
                    };
                    if line.trim() != "state" && action.is_none() {
                        "{\"error\":\"invalid_request\"}".to_owned()
                    } else {
                        let (reply, receive) = std::sync::mpsc::sync_channel(1);
                        if tx.try_send(Request { action, reply }).is_ok() {
                            receive
                                .recv_timeout(Duration::from_secs(3))
                                .unwrap_or_else(|_| "{\"error\":\"app_unresponsive\"}".into())
                        } else {
                            "{\"error\":\"app_busy\"}".into()
                        }
                    }
                } else {
                    "{\"error\":\"invalid_request\"}".into()
                };
                let _ = writeln!(stream, "{response}");
            }
            // Only this listener can own the pathname until the lock is released.
            let _ = std::fs::remove_file(path);
        })?;
    Ok(rx)
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn command(action: Option<(Action, &str)>) -> Result<String> {
    if let Some((_, id)) = action {
        ensure!(valid_id(id), "capture ID must be 32 hex characters");
    }
    let path = socket_path(false)?;
    let meta = std::fs::symlink_metadata(&path).context("no running GUI control socket")?;
    ensure!(
        meta.file_type().is_socket()
            && meta.uid() == unsafe { libc::geteuid() }
            && meta.mode() & 0o777 == 0o600,
        "unsafe control socket"
    );
    let mut stream = UnixStream::connect(path).context("GUI capture control unavailable")?;
    same_user(&stream)?;
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.set_write_timeout(Some(Duration::from_secs(4)))?;
    let request = match action {
        None => "state".to_owned(),
        Some((Action::Stop, id)) => format!("stop {id}"),
        Some((Action::Cancel, id)) => format!("cancel {id}"),
    };
    writeln!(stream, "{request}")?;
    let mut response = String::new();
    BufReader::new(stream).take(4096).read_line(&mut response)?;
    ensure!(
        !response.is_empty(),
        "GUI capture control closed without a reply"
    );
    Ok(response.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competing_launches_cannot_remove_the_winning_control_socket() {
        let dir =
            std::env::temp_dir().join(format!("apparent-agent-control-{}", new_id().unwrap()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("capture.sock");
        let stale = UnixListener::bind(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        drop(stale);

        // A launcher holding the lock may be between its stale-socket check and bind.
        // Another launcher must not unlink the socket even though connect is refused.
        let owner = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .open(path.with_extension("lock"))
            .unwrap();
        assert_eq!(unsafe { libc::flock(owner.as_raw_fd(), libc::LOCK_EX) }, 0);
        let inode = std::fs::symlink_metadata(&path).unwrap().ino();
        assert!(listen_at(path.clone()).is_err());
        assert_eq!(std::fs::symlink_metadata(&path).unwrap().ino(), inode);
        drop(owner);

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let contenders: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    listen_at(path)
                })
            })
            .collect();
        barrier.wait();
        let mut results: Vec<_> = contenders
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let receiver = results
            .remove(results.iter().position(Result::is_ok).unwrap())
            .unwrap();
        let mut stream = UnixStream::connect(&path).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        writeln!(stream, "state").unwrap();
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .reply
            .send("{\"state\":\"reachable\"}".into())
            .unwrap();
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response).unwrap();
        assert!(response.contains("reachable"));
        drop(receiver);
        for _ in 0..50 {
            if !path.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!path.exists());
        std::fs::remove_file(path.with_extension("lock")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn only_the_matching_capture_can_be_stopped_or_cancelled() {
        for phase in [Phase::Starting, Phase::Recording] {
            let state = State {
                phase: phase.clone(),
                capture_id: Some("capture-a".into()),
            };
            assert_eq!(
                decide(&state, Action::Cancel, "capture-b"),
                Outcome::WrongCapture
            );
            assert_eq!(
                decide(&state, Action::Stop, "capture-b"),
                Outcome::WrongCapture
            );
            assert_eq!(
                decide(&state, Action::Cancel, "capture-a"),
                Outcome::Cancelled
            );
            assert_eq!(
                decide(&state, Action::Stop, "capture-a"),
                if phase == Phase::Recording {
                    Outcome::Stopping
                } else {
                    Outcome::NotReady
                }
            );
        }
        for phase in [
            Phase::Idle,
            Phase::Selecting,
            Phase::Processing,
            Phase::Completed,
        ] {
            let state = State {
                phase,
                capture_id: None,
            };
            assert_eq!(
                decide(&state, Action::Stop, "capture-a"),
                Outcome::AlreadyEnded
            );
            assert_eq!(
                decide(&state, Action::Cancel, "capture-a"),
                Outcome::AlreadyEnded
            );
        }
    }

    #[test]
    fn ids_are_opaque_fresh_and_command_ids_are_strict() {
        let first = new_id().unwrap();
        let second = new_id().unwrap();
        assert!(valid_id(&first));
        assert!(valid_id(&second));
        assert_ne!(first, second);
        for invalid in [
            "",
            "capture-a",
            "0\nstop 00000000000000000000000000000000",
            "g0000000000000000000000000000000",
        ] {
            assert!(!valid_id(invalid));
            assert!(command(Some((Action::Cancel, invalid))).is_err());
        }
    }

    #[test]
    fn response_phase_and_outcome_are_machine_readable() {
        let response = serde_json::json!({
            "state": State { phase: Phase::Starting, capture_id: Some("id".into()) },
            "outcome": Outcome::NotReady,
        });
        assert_eq!(response["state"]["phase"], "starting");
        assert_eq!(response["outcome"], "not_ready");
    }
}

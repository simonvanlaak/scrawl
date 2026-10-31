use apparent_capture_engine::{Admission, Control, Runtime, Status};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const TOKEN: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "engine-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Worker(Child);
impl Worker {
    fn start(path: &Path, role: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "worker", "--nocapture"])
            .env("ENGINE_ROOT", path)
            .env("ENGINE_ROLE", role)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if line.unwrap().trim() == "READY" {
                    let _ = tx.send(());
                    break;
                }
            }
        });
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .expect("worker did not acquire lock");
        Self(child)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn worker() {
    let Ok(root) = std::env::var("ENGINE_ROOT") else {
        return;
    };
    let runtime = Runtime::open(Path::new(&root)).unwrap();
    let Admission::Acquired(_guard) = runtime.admit(A, TOKEN, B).unwrap() else {
        panic!("not admitted")
    };
    println!("READY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}
#[test]
fn competing_admission_and_killed_owner() {
    let f = Fixture::new();
    let owner = Worker::start(&f.0, "admit");
    let runtime = Runtime::open(&f.0).unwrap();
    assert!(matches!(
        runtime.admit(B, TOKEN, A).unwrap(),
        Admission::Existing
    ));
    drop(owner);
    // The rejected request is a tombstone; only a new request can acquire.
    assert!(matches!(
        runtime.admit(B, TOKEN, A).unwrap(),
        Admission::Existing
    ));
    assert!(matches!(
        runtime
            .admit("dddddddddddddddddddddddddddddddd", TOKEN, A)
            .unwrap(),
        Admission::Acquired(_)
    ));
}

#[test]
fn durable_identity_and_interrupted_receipt() {
    let f = Fixture::new();
    let owner = Worker::start(&f.0, "admit");
    let runtime = Runtime::open(&f.0).unwrap();
    assert_eq!(
        runtime.inspect(A).unwrap().unwrap().status,
        Status::Starting
    );
    assert!(
        runtime
            .request_control(A, &"e".repeat(64), Control::Cancel)
            .is_err()
    );
    assert!(runtime.request_control(B, TOKEN, Control::Cancel).is_err());
    assert!(runtime.admit(A, TOKEN, A).is_err());
    assert!(matches!(
        runtime.admit(A, TOKEN, B).unwrap(),
        Admission::Existing
    ));
    assert_eq!(
        runtime.inspect(A).unwrap().unwrap().status,
        Status::Starting
    );
    drop(owner);
    let receipt = Runtime::open(&f.0).unwrap().inspect(A).unwrap().unwrap();
    assert_eq!(receipt.status, Status::Failed);
    assert_eq!(receipt.error.as_deref(), Some("owner_interrupted"));
    assert!(receipt.bundle_path.is_none());
    assert!(matches!(
        runtime.admit(A, TOKEN, B).unwrap(),
        Admission::Existing
    ));
    assert_eq!(runtime.inspect(A).unwrap().unwrap(), receipt);
}

use super::*;
use std::{
    fs,
    process::{Child, Command},
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        // Engine rejects writable ancestors even beneath a private directory. Do not
        // weaken it or chmod the host scratch directory merely to make fixtures pass.
        let path = dirs::home_dir()
            .unwrap()
            .join(".local/share/apparent/refactor-evidence")
            .join(format!(
                "caller-test-{}",
                apparent_capture_engine::new_id().unwrap()
            ));
        paths::provision(&path, true).unwrap();
        paths::provision(&path.join("runtime"), true).unwrap();
        Self(path)
    }
    fn runtime(&self) -> Runtime {
        Runtime::open(&self.0.join("runtime")).unwrap()
    }
    fn output(&self) -> PathBuf {
        self.0.join("synthetic.apparent")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Process(Child);
impl Process {
    fn wait(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(Instant::now() < deadline, "subprocess deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn child(fixture: &Fixture, mode: &str) -> Process {
    Process(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "capture_runtime::tests::subprocess",
                "--nocapture",
            ])
            .env("APPARENT_CALLER_FIXTURE", &fixture.0)
            .env("APPARENT_CALLER_MODE", mode)
            .spawn()
            .unwrap(),
    )
}
fn wait_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < deadline, "fixture handshake deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn subprocess() {
    let Some(path) = std::env::var_os("APPARENT_CALLER_FIXTURE") else {
        return;
    };
    let path = PathBuf::from(path);
    let runtime = Runtime::open(&path.join("runtime")).unwrap();
    if std::env::var("APPARENT_CALLER_MODE").unwrap() == "contender" {
        // Same production admission decision used before each caller's media start.
        let admitted = Capture::admit_in(runtime, &path.join("contender.apparent"));
        if admitted.is_ok() {
            fs::write(path.join("audio.wav"), b"media side effect").unwrap();
            fs::write(path.join("recording.mov"), b"media side effect").unwrap();
        }
        assert!(admitted.is_err(), "busy caller entered media boundary");
    } else {
        let mut capture = Capture::admit_in(runtime, &path.join("synthetic.apparent")).unwrap();
        capture.observed_frame().unwrap();
        capture.processing().unwrap();
        fs::write(path.join("request"), &capture.request).unwrap();
        fs::write(path.join("ready"), b"processing").unwrap();
        wait_file(&path.join("release"));
        capture.cancel().unwrap();
    }
}

#[test]
fn processing_owner_survives_ui_and_excludes_other_process_and_maintenance() {
    let fixture = Fixture::new();
    let mut owner = child(&fixture, "owner");
    wait_file(&fixture.0.join("ready"));
    let request = fs::read_to_string(fixture.0.join("request")).unwrap();
    assert_eq!(
        fixture.runtime().inspect(&request).unwrap().unwrap().status,
        Status::Processing
    );
    assert!(
        fixture
            .runtime()
            .lease("synthetic", apparent_capture_engine::LeaseKind::Maintenance)
            .unwrap()
            .is_none()
    );
    let mut contender = child(&fixture, "contender");
    contender.wait();
    assert!(!fixture.0.join("audio.wav").exists());
    assert!(!fixture.0.join("recording.mov").exists());
    fs::write(fixture.0.join("release"), b"release").unwrap();
    owner.wait();
    assert_eq!(
        fixture.runtime().inspect(&request).unwrap().unwrap().status,
        Status::Cancelled
    );
    assert!(
        fixture
            .runtime()
            .lease("synthetic", apparent_capture_engine::LeaseKind::Maintenance)
            .unwrap()
            .is_some()
    );
}

#[test]
fn crash_recovers_and_wrong_owner_cannot_cancel() {
    let fixture = Fixture::new();
    let mut owner = child(&fixture, "owner");
    wait_file(&fixture.0.join("ready"));
    let request = fs::read_to_string(fixture.0.join("request")).unwrap();
    assert!(
        fixture
            .runtime()
            .request_control(
                &request,
                &apparent_capture_engine::new_owner_token().unwrap(),
                Control::Cancel
            )
            .is_err()
    );
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    let receipt = fixture.runtime().inspect(&request).unwrap().unwrap();
    assert_eq!(receipt.status, Status::Failed);
    assert_eq!(receipt.error.as_deref(), Some("owner_interrupted"));
    assert!(Capture::admit_in(fixture.runtime(), &fixture.output()).is_ok());
}

#[test]
fn publication_commits_saved_only_after_actual_artifact_validation() {
    for corrupt in [false, true] {
        let fixture = Fixture::new();
        let mut capture = Capture::admit_in(fixture.runtime(), &fixture.output()).unwrap();
        let request = capture.request.clone();
        capture.observed_frame().unwrap();
        capture.processing().unwrap();
        let result = capture.publish(|| {
            let built = crate::generate_demo_bundle(Some(fixture.output()))?;
            if corrupt {
                fs::write(
                    built.0.join("contact-sheet.png"),
                    b"broken post-build mutation",
                )?;
            }
            Ok(built)
        });
        let receipt = fixture.runtime().inspect(&request).unwrap().unwrap();
        if corrupt {
            assert!(result.is_err());
            assert_eq!(receipt.status, Status::Failed);
            assert!(receipt.bundle_path.is_none());
        } else {
            assert!(result.is_ok(), "{:#?}", result.err());
            assert_eq!(receipt.status, Status::Saved);
            assert_eq!(receipt.bundle_path, Some(fixture.output()));
        }
    }
}

#[test]
fn unsafe_ancestor_is_refused_without_chmod_or_media() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let parent = fixture.0.join("unsafe");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o775)).unwrap();
    assert!(Capture::admit_in(fixture.runtime(), &parent.join("unsafe.apparent")).is_err());
    assert_eq!(
        fs::metadata(&parent).unwrap().permissions().mode() & 0o777,
        0o775
    );
    assert!(!parent.join("unsafe.apparent").exists());
}

#[test]
fn replay_staging_publishes_without_linking_or_changing_source_movie() {
    use crate::model::{RgbaFrame, TranscriptData};
    use std::os::unix::fs::MetadataExt;
    let fixture = Fixture::new();
    let source = fixture.0.join("recording.mp4");
    let original = include_bytes!("../../tests/fixtures/publication/tiny.mp4");
    fs::write(&source, original).unwrap();
    let (_, demo, _) =
        crate::generate_demo_bundle(Some(fixture.0.join("template.apparent"))).unwrap();
    let output = fixture.output();
    let staged = crate::stage_replay_movie(&source, &output).unwrap();
    assert_eq!(
        fs::metadata(&source).unwrap().dev(),
        fs::metadata(&staged).unwrap().dev()
    );
    let result = crate::build_bundle_from_parts(
        &output,
        demo.target,
        1000,
        vec![RgbaFrame::new(0, 32, 32, vec![128; 32 * 32 * 4])],
        vec![],
        vec![],
        TranscriptData::default(),
        Some(staged.clone()),
        None,
        crate::SourceExtras::default(),
    );
    assert!(
        result.is_ok(),
        "real replay staging failed publication: {:#?}",
        result.err()
    );
    assert_ne!(
        fs::metadata(&source).unwrap().ino(),
        fs::metadata(&staged).unwrap().ino()
    );
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::metadata(&source).unwrap().nlink(), 1);
}

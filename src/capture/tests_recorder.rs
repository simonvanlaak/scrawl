use super::*;
use crate::model::{CaptureMode, RgbaFrame, TargetFrame};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn root_capture_leaves_have_engine_type_identity() {
    use apparent_capture_engine::{model, recording};
    use std::any::TypeId;

    assert_eq!(
        TypeId::of::<TargetInfo>(),
        TypeId::of::<model::TargetInfo>()
    );
    assert_eq!(
        TypeId::of::<CaptureMode>(),
        TypeId::of::<model::CaptureMode>()
    );
    assert_eq!(
        TypeId::of::<TargetFrame>(),
        TypeId::of::<model::TargetFrame>()
    );
    assert_eq!(TypeId::of::<RgbaFrame>(), TypeId::of::<model::RgbaFrame>());
    assert_eq!(
        TypeId::of::<CandidateBuffer>(),
        TypeId::of::<recording::CandidateBuffer>()
    );
    assert_eq!(
        TypeId::of::<ScreenRecording>(),
        TypeId::of::<recording::ScreenRecording>()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn producer_terminal_source_error_is_not_pending_consent() {
    // Runs the actual Linux sampler and source resolver. An invalid selected window is
    // refused before X11, D-Bus, microphone or any screen capture is opened.
    let target = TargetInfo {
        mode: CaptureMode::Window,
        app: None,
        bundle_id: None,
        window_title: None,
        window_id: None,
        display_id: 0,
        frame: TargetFrame {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            scale: 1.0,
        },
    };
    let session = ScreenCaptureSession::start(target, None, Arc::new(Mutex::new(vec![])));
    let deadline = Instant::now() + Duration::from_secs(2);
    while !session.failed_to_start() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let failed = session.failed_to_start();
    assert!(!session.has_frames());
    // Explicit caller cleanup, not an Inspect side effect.
    let recording = session.stop();
    assert!(recording.frames.is_empty());
    assert!(recording.video.is_none());
    assert!(
        failed,
        "the source resolver returned a terminal error, not pending consent"
    );
}

#[test]
fn transient_screenshot_failure_retries_to_readiness_and_finalization() {
    for error in [
        "screenshot acquisition failed",
        "screenshot has invalid pixel length",
    ] {
        let (sender, receiver) = std::sync::mpsc::sync_channel(0);
        let session = ScreenCaptureSession::start_with_runner(
            target(),
            None,
            Arc::new(Mutex::new(vec![])),
            move |sampler| {
                sender
                    .send((
                        Arc::clone(&sampler.frames),
                        Arc::clone(&sampler.real_frame_seen),
                        Arc::clone(&sampler.startup_failed),
                    ))
                    .unwrap();
                while !sampler.stopping.load(Ordering::Acquire) {
                    thread::park_timeout(Duration::from_millis(5));
                }
                None
            },
        );
        let (frames, seen, failed) = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        sample_real_candidate(&frames, &seen, &failed, true, || Err(error.into()));
        assert!(
            !session.has_frames(),
            "a failed sample cannot supply readiness"
        );
        let candidates =
            std::mem::replace(&mut *frames.lock().unwrap(), CandidateBuffer::new(1 << 20));
        assert!(candidates.into_frames().is_empty(), "no demo pixels");
        let mut retried = false;
        sample_real_candidate(&frames, &seen, &failed, false, || {
            retried = true;
            Ok(RgbaFrame::new(220, 64, 64, vec![255; 64 * 64 * 4]))
        });
        assert!(
            retried,
            "{error}: an individual sample failure must not disable later attempts"
        );
        assert!(
            session.has_frames(),
            "the next successful sample unlocks Stop"
        );
        assert!(!session.failed_to_start());
        let recording = session.stop();
        assert_eq!(recording.frames.len(), 1);
        let dir = std::env::temp_dir().join(crate::make_bundle_id());
        let target = TargetInfo {
            mode: CaptureMode::Region,
            app: None,
            bundle_id: None,
            window_title: None,
            window_id: None,
            display_id: 0,
            frame: TargetFrame {
                x: 0.0,
                y: 0.0,
                width: 64.0,
                height: 64.0,
                scale: 1.0,
            },
        };
        let (saved, bundle, _) = crate::finalize_capture_bundle(
            &dir,
            target,
            recording,
            vec![],
            vec![],
            None,
            None,
            crate::windows::ActivityLog::default(),
            None,
        )
        .expect("a recovered real sample must allow ordinary finalization");
        assert_eq!(saved, dir);
        assert!(!bundle.keyframes.is_empty());
        assert!(dir.join("apparent.json").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

fn target() -> TargetInfo {
    TargetInfo {
        mode: CaptureMode::Region,
        app: None,
        bundle_id: None,
        window_title: None,
        window_id: None,
        display_id: 0,
        frame: TargetFrame {
            x: 0.0,
            y: 0.0,
            width: 64.0,
            height: 64.0,
            scale: 1.0,
        },
    }
}

#[test]
fn desktop_adapter_keeps_real_retention_until_worker_teardown() {
    // Isolate the actual process-wide retention counter from other recording tests.
    const CHILD: &str = "APPARENT_RECORDER_RETENTION_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "capture::recorder::tests::desktop_adapter_keeps_real_retention_until_worker_teardown", "--nocapture"])
            .env(CHILD, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    let dir = std::env::temp_dir().join(crate::make_bundle_id());
    std::fs::create_dir(&dir).unwrap();
    let actions = vec![crate::retention::Action::Shrink {
        bundle: dir.clone(),
        video: dir.join("missing.mov"),
    }];
    let check = move || {
        crate::retention::apply(
            &actions,
            &crate::retention::Policy::default(),
            &|_| panic!("shrink must not trash"),
            true,
        )
        .errors
    };
    // The positive control reaches the real shrink/probe path and fails on missing media.
    assert_eq!(
        check(),
        1,
        "retention check must not silently skip ownership"
    );
    for stop in [true, false] {
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker_check = check.clone();
        let session = ScreenCaptureSession::start_with_runner(
            target(),
            None,
            Arc::new(Mutex::new(vec![17])),
            move |sampler| {
                let entered = worker_check();
                sender.send((false, entered)).unwrap();
                let deadline = Instant::now() + Duration::from_secs(10);
                while !sampler.stopping.load(Ordering::Acquire) && Instant::now() < deadline {
                    thread::park_timeout(Duration::from_secs(10));
                }
                let protected = worker_check();
                push_candidate(
                    &sampler.frames,
                    &sampler.real_frame_seen,
                    RgbaFrame::new(42, 1, 1, vec![1, 2, 3, 255]),
                    false,
                    true,
                );
                sender
                    .send((sampler.stopping.load(Ordering::Acquire), protected))
                    .unwrap();
                None
            },
        );
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
            (false, 0)
        );
        assert!(!session.has_frames());
        assert!(!session.failed_to_start());
        if stop {
            let (recording, seen) = session.stop_with_readiness();
            assert!(seen);
            assert_eq!(recording.frames.len(), 1);
            assert!(recording.duration_ms >= 100);
        } else {
            drop(session);
        }
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
            (true, 0)
        );
        assert_eq!(
            check(),
            1,
            "the real guard must release after join/collection"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

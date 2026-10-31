//! Production-entry regressions: invalid persisted bytes must not escape as success.
use super::*;

fn target() -> TargetInfo {
    serde_json::from_value(serde_json::json!({
        "mode": "window", "displayId": 1,
        "frame": {"x":0.0,"y":0.0,"width":32.0,"height":32.0,"scale":1.0}
    }))
    .unwrap()
}

#[test]
fn publication_rejects_corrupt_prepared_png_before_success() {
    let root = std::env::temp_dir().join(crate::make_bundle_id());
    let frame = RgbaFrame::new(0, 32, 32, vec![128; 32 * 32 * 4]);
    let mut marks = vec![];
    let selected = select_keyframes(
        &[frame.clone()],
        &[],
        &mut marks,
        &[],
        &TranscriptData::default(),
        6,
    );
    let mut prepared = prepare_visuals(
        selected,
        Some(&frame),
        None,
        &target(),
        1000,
        &[],
        &[],
        &[],
        &windows::ActivityLog::default(),
    )
    .unwrap();
    prepared.contact_sheet_png = b"not a PNG".to_vec();
    let result = build_bundle_from_parts(
        &root,
        target(),
        1000,
        vec![frame],
        vec![],
        vec![],
        TranscriptData::default(),
        None,
        None,
        SourceExtras {
            prepared: Some(prepared),
            autocrop: Some(None),
            ..Default::default()
        },
    );
    let success_effect = result
        .as_ref()
        .ok()
        .map(|_| "Saved/clipboard/upload eligible");
    assert!(
        success_effect.is_none(),
        "invalid prepared bytes escaped the production finalizer"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_checks_wav_persistence_before_transcription_or_manifest() {
    let root = std::env::temp_dir().join(crate::make_bundle_id());
    std::fs::create_dir_all(root.join("audio.wav")).unwrap();
    let recorded =
        audio::recorded_audio_from_wav(audio::encode_wav_16k(&[0.0; 16]).unwrap()).unwrap();
    let result = finalize_capture_bundle(
        &root,
        target(),
        capture::ScreenRecording {
            frames: vec![RgbaFrame::new(0, 32, 32, vec![128; 32 * 32 * 4])],
            video: None,
            duration_ms: 1,
            video_offset_ms: 0,
        },
        vec![],
        vec![],
        Some(recorded),
        None,
        windows::ActivityLog::default(),
        None,
    );
    assert!(result.is_err());
    assert!(
        !root.join("apparent.json").exists(),
        "WAV write failure must stop before manifest publication"
    );
    std::fs::remove_dir_all(root).unwrap();
}

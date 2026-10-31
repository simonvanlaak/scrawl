use super::*;
use crate::model::{CaptureMode, TargetFrame, TargetInfo};
use serde_json::json;

fn frame(t_ms: u64) -> RgbaFrame {
    RgbaFrame::new(t_ms, 1, 1, vec![1, 2, 3, 4])
}

fn times(buffer: CandidateBuffer) -> Vec<u64> {
    buffer.into_frames().into_iter().map(|f| f.t_ms).collect()
}

#[test]
fn target_wire_schema_preserves_names_defaults_and_all_modes() {
    let geometry = json!({"x": -12.5, "y": 20.0, "width": 80.0, "height": 60.0, "scale": 2.0});
    for (mode, name) in [
        (CaptureMode::Display, "display"),
        (CaptureMode::Window, "window"),
        (CaptureMode::Region, "region"),
        (CaptureMode::Device, "device"),
    ] {
        let minimal = json!({"mode": name, "displayId": 42, "frame": geometry});
        let target: TargetInfo = serde_json::from_value(minimal.clone()).unwrap();
        assert_eq!(target.mode, mode);
        assert_eq!(
            (
                target.app.as_ref(),
                target.bundle_id.as_ref(),
                target.window_title.as_ref(),
                target.window_id
            ),
            (None, None, None, None)
        );
        assert_eq!(serde_json::to_value(&target).unwrap(), minimal);
        let full = TargetInfo {
            app: Some("Example".into()),
            bundle_id: Some("org.example.App".into()),
            window_title: Some("Window".into()),
            window_id: Some(17),
            ..target
        };
        let wire = json!({"mode": name, "displayId": 42, "frame": geometry,
            "app": "Example", "bundleId": "org.example.App", "windowTitle": "Window", "windowId": 17});
        assert_eq!(serde_json::to_value(&full).unwrap(), wire);
        assert_eq!(serde_json::from_value::<TargetInfo>(wire).unwrap(), full);
    }
    assert!(
        serde_json::from_value::<TargetInfo>(json!({"mode":"display", "frame":geometry})).is_err()
    );
    assert!(serde_json::from_value::<CaptureMode>(json!("Display")).is_err());
    let parsed: TargetFrame = serde_json::from_value(geometry).unwrap();
    assert_eq!(
        (
            parsed.x,
            parsed.y,
            parsed.width,
            parsed.height,
            parsed.scale
        ),
        (-12.5, 20.0, 80.0, 60.0, 2.0)
    );
}

#[test]
fn rgba_and_finished_recording_preserve_owned_pixels_and_metadata() {
    let mut pixels = frame(37);
    assert!(!pixels.badge_bottom_left);
    pixels.badge_bottom_left = true;
    let recording = ScreenRecording {
        frames: vec![pixels.clone()],
        video: Some(PathBuf::from("recording.mp4")),
        duration_ms: 500,
        video_offset_ms: 29,
    };
    pixels.rgba[0] = 9;
    let saved = &recording.frames[0];
    assert_eq!((saved.t_ms, saved.width, saved.height), (37, 1, 1));
    assert_eq!(saved.rgba, [1, 2, 3, 4]);
    assert!(saved.badge_bottom_left);
    assert_eq!(recording.video, Some(PathBuf::from("recording.mp4")));
    assert_eq!(
        (recording.duration_ms, recording.video_offset_ms),
        (500, 29)
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic]
fn rgba_constructor_checks_pixel_length_in_debug_builds() {
    RgbaFrame::new(0, 2, 1, vec![0; 4]);
}

#[test]
fn regular_frames_thin_at_budget_then_double_sampling_interval() {
    let mut buffer = CandidateBuffer::new(240);
    for t in 0..59 {
        buffer.push(frame(t), false);
    }
    assert_eq!(buffer.len(), 59);
    assert_eq!(buffer.interval(), Duration::from_millis(220));
    buffer.push(frame(59), false);
    assert_eq!(buffer.len(), 30);
    assert_eq!(buffer.interval(), Duration::from_millis(440));
    for t in 60..90 {
        buffer.push(frame(t), false);
    }
    assert_eq!(buffer.interval(), Duration::from_millis(880));
    let mut expected: Vec<u64> = (0..60).step_by(4).collect();
    expected.extend((60..90).step_by(2));
    assert_eq!(times(buffer), expected);
}

#[test]
fn isolated_urgent_frames_do_not_advance_regular_parity() {
    let mut buffer = CandidateBuffer::new(240);
    for t in 0..60 {
        buffer.push(frame(t), matches!(t, 1 | 4));
    }
    let mut expected = vec![0, 1, 3, 4];
    expected.extend((6..60).step_by(2));
    assert_eq!(times(buffer), expected);
}

#[test]
fn urgent_storms_are_also_thinned_but_keep_the_first_frame() {
    let mut buffer = CandidateBuffer::new(0);
    for t in 0..60 {
        buffer.push(frame(t), true);
    }
    assert_eq!(buffer.interval(), Duration::from_millis(440));
    assert_eq!(times(buffer), (0..60).step_by(2).collect::<Vec<_>>());
}

#[test]
fn storm_second_pass_starts_only_above_three_quarters_capacity() {
    for (urgent_count, expected_len) in [(30, 45), (32, 23)] {
        let mut buffer = CandidateBuffer::new(240);
        for t in 0..60 {
            buffer.push(frame(t), t < urgent_count);
        }
        assert_eq!(buffer.len(), expected_len);
        assert_eq!(times(buffer)[0], 0);
    }
}

#[test]
fn capacity_uses_first_frame_bytes_and_clamps_to_sixty_through_twelve_hundred() {
    for budget in [0, 1, 239] {
        let mut buffer = CandidateBuffer::new(budget);
        for t in 0..59 {
            buffer.push(frame(t), false);
        }
        assert_eq!(buffer.len(), 59);
        buffer.push(frame(59), false);
        assert_eq!(buffer.len(), 30);
    }
    let mut buffer = CandidateBuffer::new(usize::MAX);
    // Empty pixel payloads use a one-byte divisor, not division by zero.
    buffer.push(RgbaFrame::new(0, 0, 0, vec![]), false);
    for t in 1..1199 {
        buffer.push(frame(t), false);
    }
    assert_eq!(buffer.len(), 1199);
    buffer.push(frame(1199), false);
    assert_eq!(buffer.len(), 600);
    assert_eq!(buffer.interval(), Duration::from_millis(440));
    let mut first_sized = CandidateBuffer::new(480);
    first_sized.push(RgbaFrame::new(0, 2, 1, vec![0; 8]), false);
    for t in 1..60 {
        first_sized.push(frame(t), false);
    }
    assert_eq!(first_sized.len(), 30);
}

#[test]
fn drop_since_is_inclusive_and_always_preserves_first_insertion() {
    let mut buffer = CandidateBuffer::new(240);
    for t in [100, 10, 20, 30] {
        buffer.push(frame(t), true);
    }
    buffer.drop_since(20);
    assert_eq!(buffer.interval(), FRAME_SAMPLE_INTERVAL);
    assert_eq!(times(buffer), [100, 10, 20]);
    let mut buffer = CandidateBuffer::new(0);
    buffer.drop_since(0);
    assert!(times(buffer).is_empty());
}

// Preparation-only compatibility receipt: compare complete leaf blocks, not line numbers,
// identifier counts or a second transcription of the algorithm. These checkout-relative
// includes work on both supported OSes. Remove this donor check with paired adoption.
#[test]
fn staged_leaf_definitions_are_byte_identical_to_root_donors() {
    fn block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let (_, tail) = source.split_once(start).expect("leaf start");
        tail.split_once(end).expect("leaf end").0
    }
    let donor_model = include_str!("../../../../src/model.rs");
    let staged_model = include_str!("../model.rs");
    let target_start = "#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]\n#[serde(rename_all = \"camelCase\")]\npub struct TargetInfo";
    assert_eq!(
        block(
            donor_model,
            target_start,
            "#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]"
        ),
        block(staged_model, target_start, "/// Raw RGBA8 frame buffer"),
    );
    let rgba_start = "/// Raw RGBA8 frame buffer";
    assert_eq!(
        block(donor_model, rgba_start, "\n#[cfg(test)]"),
        staged_model.split_once(rgba_start).unwrap().1,
    );
    let donor_recorder = include_str!("../../../../src/capture/recorder.rs");
    let staged_recorder = include_str!("mod.rs");
    let constants_start = "pub const FRAME_SAMPLE_INTERVAL";
    assert_eq!(
        block(donor_recorder, constants_start, "/// The capture size for"),
        block(staged_recorder, constants_start, "#[cfg(test)]\nmod tests;"),
    );
    assert_eq!(
        (
            CANDIDATE_EDGE,
            CANDIDATE_EDGE_NO_VIDEO,
            CANDIDATE_BUDGET_BYTES
        ),
        (480.0, 1280.0, 384 << 20)
    );
}

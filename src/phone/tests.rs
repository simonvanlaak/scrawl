use super::*;

/// An inert protocol producer, not the native helper. It cannot acknowledge its
/// first frame until stop writes the command; the real production reader joins it.
pub(crate) fn session_after_command(
    sender: flume::Sender<LiveOverlayEvent>,
    acknowledge: bool,
) -> PhoneSession {
    let payload = if acknowledge {
        concat!(
            "{\"event\":\"device\",\"id\":\"fixture\",\"name\":\"Synthetic phone\",\"model\":\"fixture\",\"width\":16,\"height\":16}\n",
            "{\"event\":\"recording\",\"startedAt\":1}\n",
            "{\"event\":\"finished\",\"path\":\"synthetic.mov\",\"durationMs\":200}\n"
        )
    } else {
        "{\"event\":\"cancelled\"}\n"
    };
    let mut child = Command::new("sh")
        .args(["-c", "read command; printf '%s' \"$1\"", "fixture", payload])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take();
    let output = child.stdout.take().unwrap();
    let state = Arc::new(Mutex::new(MirrorState::default()));
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let read_state = Arc::clone(&state);
    let read_cancelled = Arc::clone(&cancelled);
    let end_identity = Arc::new(());
    let reader_identity = Arc::clone(&end_identity);
    let telemetry = SharedCaptureTelemetry::new(sender);
    let reader = std::thread::spawn(move || {
        reader::read_events(
            BufReader::new(output),
            read_state,
            read_cancelled,
            reader_identity,
            Some(telemetry),
        );
    });
    PhoneSession {
        child: Some(child),
        stdin,
        reader: Some(reader),
        state,
        cancelled,
        end_identity,
        started_at: Instant::now(),
        session_epoch_ms: 0,
        busy: None,
    }
}

#[test]
fn review40_intentional_phone_teardown_cannot_queue_an_end_for_replacement() {
    // Direct CLI stop, recording wrapper stop/discard, wrapper destruction, and
    // ordinary preview cancellation/drop all must suppress unsolicited events.
    for teardown in 0..5 {
        let (sender, receiver) = flume::unbounded();
        let a = session_after_command(sender.clone(), false);
        match teardown {
            0 => {
                let _ = a.stop();
            }
            1 => {
                let _ = crate::capture_runtime::Phone::new(a).stop();
            }
            2 => drop(crate::capture_runtime::Phone::new(a)),
            3 => a.cancel(),
            _ => drop(a),
        }
        let b = session_after_command(sender, false);
        // An event already queued by A would be dispatched against B's state.
        assert!(
            receiver.try_recv().is_err(),
            "teardown {teardown} queued a stale PhoneEnded for B"
        );
        assert!(!b.has_ended());
        drop(b);
    }
}

#[test]
fn review40_phone_stop_returns_first_acknowledgement_read_during_join() {
    for wrapped in [false, true] {
        let (sender, _receiver) = flume::unbounded();
        let session = session_after_command(sender, true);
        assert!(!session.has_recording());
        let (recording, frame_seen) = if wrapped {
            crate::capture_runtime::Phone::new(session).stop_with_readiness()
        } else {
            session.stop_with_readiness()
        };
        assert!(recording.is_ok());
        assert!(
            frame_seen,
            "post-join acknowledgement lost (recording wrapper={wrapped})"
        );
    }
    let (sender, _receiver) = flume::unbounded();
    let (recording, frame_seen) = session_after_command(sender, false).stop_with_readiness();
    assert!(recording.is_err());
    assert!(
        !frame_seen,
        "cancelled/no-frame helper must not become ready"
    );
}

#[test]
fn review40_unexpected_phone_end_still_notifies() {
    let (sender, receiver) = flume::unbounded();
    let mut session = session_after_command(sender, false);
    // A helper that ends without stop/cancel: unblock the inert process directly.
    writeln!(session.stdin.as_mut().unwrap(), "unexpected").unwrap();
    session.reader.take().unwrap().join().unwrap();
    assert!(matches!(
        receiver.try_recv(),
        Ok(LiveOverlayEvent::PhoneEnded(_))
    ));
    assert!(session.has_ended());
}

pub(crate) fn end_unexpectedly(mut session: PhoneSession) -> PhoneSession {
    session.send("unexpected");
    session.reader.take().unwrap().join().unwrap();
    session
}

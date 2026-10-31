//! Transcribing while recording (`"experiments": {"liveTranscribe": true}`, Apple Speech only).
//!
//! A long-lived `apparent-helper transcribe-stream` gets the microphone's 16 kHz samples, quantized
//! exactly as `audio.wav` will store them, about every 160 ms, and keeps SpeechAnalyzer's
//! finalized results. At finish the samples it hasn't had are sent from the finished WAV and only
//! that tail is left to transcribe, so the words and times are the WAV's own. Anything wrong (the
//! helper fails, the model isn't installed, it lost samples or took too long) returns an error and
//! the caller transcribes the WAV as before.

use std::io::{BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::Deserialize;

use crate::model::{TranscriptData, TranscriptSegment, WordTiming};

const FEED_EVERY: Duration = Duration::from_millis(160);
/// How long finishing waits for the tail before giving up on the stream. The analyzer keeps up
/// with speech in real time, so the tail normally takes well under 100 ms; a model still loading
/// when a short recording ends takes longer, as it would after the recording too.
const TAIL_TIMEOUT: Duration = Duration::from_secs(6);

/// Whether recordings should start a live transcription now. It streams to `apparent-helper`'s
/// SpeechAnalyzer, so only on macOS (Windows transcribes after the recording).
pub fn wanted() -> bool {
    cfg!(target_os = "macos")
        && crate::handoff::experiment(crate::handoff::LIVE_TRANSCRIBE)
        && crate::transcribe::Engine::preferred() == crate::transcribe::Engine::Apple
}

pub struct Live {
    stop: Arc<AtomicBool>,
    feeder: Option<JoinHandle<Result<Feeding>>>,
}

struct Feeding {
    child: Child,
    stdin: Option<ChildStdin>,
    fed: usize,
    lines: Receiver<Line>,
}

impl Drop for Feeding {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: start_with gives the helper its own process group, retained until reaped.
        unsafe {
            libc::killpg(self.child.id() as libc::pid_t, libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Event {
    Done {
        done: bool,
        samples: usize,
    },
    Ready {
        ready: bool,
        locale: String,
        // Older helpers only streamed SpeechTranscriber and did not name it.
        #[serde(default = "default_engine")]
        engine: String,
    },
    Result {
        segment: TranscriptSegment,
        #[serde(default)]
        words: Vec<WordTiming>,
    },
}

enum Line {
    Event(Event),
    Invalid,
    /// stdout closed; the helper's stderr.
    Closed(String),
}

fn default_engine() -> String {
    "SpeechTranscriber".into()
}

/// What finishing took, for the log.
#[derive(Debug)]
pub struct Finished {
    pub transcript: TranscriptData,
    pub tail_ms: u64,
    pub streamed_samples: usize,
}

impl Live {
    /// Starts streaming `samples` (the recorder's 16 kHz buffer, which only grows until the
    /// recording stops) to a new helper. Nothing here blocks: the helper starts on the feeder
    /// thread.
    pub fn start(samples: Arc<Mutex<Vec<f32>>>) -> Result<Self> {
        let mut command = Command::new(crate::helper::path()?);
        command.arg("transcribe-stream");
        command.args(crate::speech_model::locale_args());
        Self::start_with(command, samples)
    }

    fn start_with(mut command: Command, samples: Arc<Mutex<Vec<f32>>>) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let feeder_stop = Arc::clone(&stop);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);
        let feeder = std::thread::Builder::new()
            .name("apparent-live-transcribe".into())
            .spawn(move || feed(command, &samples, &feeder_stop))
            .context("Could not spawn the live transcription thread")?;
        Ok(Self {
            stop,
            feeder: Some(feeder),
        })
    }

    /// Sends what the helper hasn't had of `wav_bytes` (the recording's `audio.wav`), ends its
    /// input and collects the transcript.
    pub fn finish(mut self, wav_bytes: &[u8]) -> Result<Finished> {
        self.finish_within(wav_bytes, TAIL_TIMEOUT)
    }

    fn finish_within(&mut self, wav_bytes: &[u8], timeout: Duration) -> Result<Finished> {
        let finishing = Instant::now();
        let deadline = finishing + timeout;
        let mut feeding = self.join_feeder()?;
        let samples = crate::audio::decode_wav_pcm16(wav_bytes)?;
        ensure!(
            feeding.fed <= samples.len(),
            "streamed {} samples but the recording has {}",
            feeding.fed,
            samples.len()
        );
        let streamed = feeding.fed;
        {
            let mut stdin = feeding.stdin.take().context("No helper stdin")?;
            write_pcm(&mut stdin, &le_bytes(&samples[feeding.fed..]), || {
                ensure!(
                    Instant::now() < deadline,
                    "the live transcriber took too long to finish"
                );
                Ok(false)
            })?;
        }
        let mut locale = None;
        let mut engine = None;
        let mut segments = Vec::new();
        let mut words = Vec::new();
        loop {
            ensure!(
                Instant::now() < deadline,
                "the live transcriber took too long to finish"
            );
            let left = deadline.saturating_duration_since(Instant::now());
            match feeding.lines.recv_timeout(left) {
                Ok(Line::Event(Event::Ready {
                    ready: true,
                    locale: l,
                    engine: e,
                })) if locale.is_none()
                    && !l.is_empty()
                    && matches!(e.as_str(), "SpeechTranscriber" | "DictationTranscriber") =>
                {
                    locale = Some(l);
                    engine = Some(e);
                }
                Ok(Line::Event(Event::Result {
                    segment,
                    words: more,
                })) => {
                    segments.push(segment);
                    words.extend(more);
                }
                Ok(Line::Event(Event::Done {
                    done: true,
                    samples: heard,
                })) => {
                    ensure!(
                        heard == samples.len(),
                        "the live transcriber heard {heard} samples of {}",
                        samples.len()
                    );
                    break;
                }
                Ok(Line::Invalid | Line::Event(_)) => bail!("invalid live transcription protocol"),
                Ok(Line::Closed(stderr)) => bail!(
                    "the live transcriber stopped: {}",
                    stderr.trim().trim_start_matches("apparent-helper: ")
                ),
                Err(RecvTimeoutError::Timeout) => {
                    bail!(
                        "the live transcriber took over {} s to finish",
                        timeout.as_secs()
                    )
                }
                Err(RecvTimeoutError::Disconnected) => bail!("the live transcriber vanished"),
            }
        }
        let locale = locale.context("the live transcriber never said it was ready")?;
        let duration_ms = (samples.len() as u64 * 1000).div_ceil(16_000);
        ensure!(
            segments.iter().all(|s| s.t0 <= s.t1 && s.t1 <= duration_ms),
            "invalid live segment times"
        );
        ensure!(
            words.iter().all(|w| w.t0 <= w.t1 && w.t1 <= duration_ms),
            "invalid live word times"
        );
        Ok(Finished {
            transcript: transcript(
                &engine.context("the live transcriber never named its engine")?,
                &locale,
                segments,
                words,
            ),
            tail_ms: finishing.elapsed().as_millis() as u64,
            streamed_samples: streamed,
        })
    }

    fn join_feeder(&mut self) -> Result<Feeding> {
        self.stop.store(true, Ordering::Release);
        let feeder = self.feeder.take().context("Already finished")?;
        feeder.thread().unpark();
        feeder
            .join()
            .map_err(|_| anyhow!("The live transcription thread panicked"))?
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        if self.feeder.is_some() {
            let _ = self.join_feeder();
        }
    }
}

/// As `helper::transcribe` builds it from the batch helper's output.
fn transcript(
    engine: &str,
    locale: &str,
    mut segments: Vec<TranscriptSegment>,
    mut words: Vec<WordTiming>,
) -> TranscriptData {
    segments.sort_by_key(|s| s.t0);
    words.sort_by_key(|w| w.t0);
    TranscriptData {
        source: "speech-analyzer".into(),
        model: Some(format!("{engine} {locale}")),
        text: segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        segments,
        words,
        estimated: false,
        missing: None,
    }
}

fn feed(mut command: Command, samples: &Mutex<Vec<f32>>, stop: &AtomicBool) -> Result<Feeding> {
    let mut child = command
        .spawn()
        .context("Could not start apparent-helper transcribe-stream")?;
    let stdin = child.stdin.take().context("No helper stdin")?;
    let stdout = child.stdout.take().context("No helper stdout")?;
    let mut stderr = child.stderr.take().context("No helper stderr")?;
    let (sender, lines) = std::sync::mpsc::channel();
    // Acquire cleanup ownership before fallible thread/pipe setup.
    let mut feeding = Feeding {
        child,
        stdin: Some(stdin),
        fed: 0,
        lines,
    };
    nonblocking(feeding.stdin.as_ref().context("No helper stdin")?)?;
    let (errors, error_text) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("apparent-live-transcribe-err".into())
        .spawn(move || {
            let mut message = Vec::new();
            let mut chunk = [0; 4096];
            while let Ok(n) = stderr.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let keep = n.min(8192_usize.saturating_sub(message.len()));
                message.extend_from_slice(&chunk[..keep]);
            }
            let _ = errors.send(String::from_utf8_lossy(&message).into_owned());
        })
        .context("Could not spawn the live transcription error reader")?;
    std::thread::Builder::new()
        .name("apparent-live-transcribe-out".into())
        .spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match serde_json::from_str::<Event>(&line) {
                    Ok(event) => {
                        let _ = sender.send(Line::Event(event));
                    }
                    Err(_) => {
                        let _ = sender.send(Line::Invalid);
                    }
                }
            }
            let message = error_text
                .recv_timeout(Duration::from_millis(100))
                .unwrap_or_default();
            let _ = sender.send(Line::Closed(message));
        })
        .context("Could not spawn the live transcription reader")?;
    let mut bytes = Vec::new();
    while !stop.load(Ordering::Acquire) {
        bytes.clear();
        {
            let buffer = match samples.try_lock() {
                Ok(buffer) => buffer,
                Err(TryLockError::WouldBlock) => {
                    std::thread::park_timeout(Duration::from_millis(5));
                    continue;
                }
                Err(TryLockError::Poisoned(_)) => bail!("Microphone buffer lock poisoned"),
            };
            // The recorder empties the buffer when it stops; the rest comes from the WAV.
            if let Some(fresh) = buffer.get(feeding.fed..) {
                bytes.extend(
                    fresh
                        .iter()
                        .take(16_000)
                        .flat_map(|&s| crate::audio::pcm16(s).to_le_bytes()),
                );
            }
        }
        if !bytes.is_empty() {
            let stdin = feeding.stdin.as_mut().context("No helper stdin")?;
            feeding.fed += write_pcm(stdin, &bytes, || Ok(stop.load(Ordering::Acquire)))? / 2;
        }
        std::thread::park_timeout(FEED_EVERY);
    }
    Ok(feeding)
}

/// A stalled model must not hold Finish or Cancel hostage in a pipe write.
fn nonblocking(stdin: &ChildStdin) -> Result<()> {
    #[cfg(unix)]
    {
        let fd = stdin.as_raw_fd();
        // SAFETY: stdin owns this live descriptor; only its status flags are changed.
        let ok = unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            flags != -1 && libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) != -1
        };
        ensure!(ok, "could not make the live transcription pipe nonblocking");
        Ok(())
    }
    #[cfg(not(unix))]
    bail!("live transcription requires a Unix pipe")
}

fn write_pcm(
    stdin: &mut ChildStdin,
    bytes: &[u8],
    mut stop: impl FnMut() -> Result<bool>,
) -> Result<usize> {
    let mut written = 0;
    while written < bytes.len() {
        if stop()? {
            break;
        }
        // <= POSIX PIPE_BUF: nonblocking writes are atomic, so cancellation leaves whole i16s.
        let end = (written + 512).min(bytes.len());
        match stdin.write(&bytes[written..end]) {
            Ok(0) => bail!("The live transcriber closed its input"),
            Ok(n) => written += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e).context("The live transcriber stopped"),
        }
    }
    Ok(written)
}

fn le_bytes(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

/// Plays `wav_bytes` into a live transcription as a microphone would, `speed` times real time in
/// 10 ms callbacks, and returns it ready to finish (`apparent reprocess --bench N --live-speed S`).
pub fn simulate(wav_bytes: &[u8], speed: f64) -> Result<Live> {
    let samples = crate::audio::decode_wav_16k(wav_bytes)?;
    let buffer = Arc::new(Mutex::new(Vec::with_capacity(samples.len())));
    let live = Live::start(Arc::clone(&buffer))?;
    let started = Instant::now();
    for (i, chunk) in samples.chunks(160).enumerate() {
        buffer
            .lock()
            .map_err(|_| anyhow!("poisoned"))?
            .extend_from_slice(chunk);
        let due = Duration::from_secs_f64((i + 1) as f64 * 0.01 / speed.max(0.01));
        if let Some(wait) = due.checked_sub(started.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    // The recorder empties its buffer when it stops.
    buffer.lock().map_err(|_| anyhow!("poisoned"))?.clear();
    Ok(live)
}

/// Shares a live transcription between copies of a recording; the first `take` gets it.
#[derive(Clone, Default)]
pub struct Slot(Arc<Mutex<Option<Live>>>);

impl Slot {
    pub fn new(live: Option<Live>) -> Self {
        Self(Arc::new(Mutex::new(live)))
    }

    pub fn take(&self) -> Option<Live> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let live = self.0.lock().map(|live| live.is_some()).unwrap_or(false);
        f.debug_tuple("Slot").field(&live).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn finite_chunks_keep_absolute_source_times() {
        let script = format!(
            r#"{READY}; cat >/dev/null; echo '{{"segment":{{"t0":63,"t1":150,"text":"First."}},"words":[{{"t0":63,"t1":150,"text":"First."}}]}}'; echo '{{"segment":{{"t0":913,"t1":1000,"text":"Tail."}},"words":[{{"t0":913,"t1":1000,"text":"Tail."}}]}}'; echo '{{"done":true,"samples":16000}}'"#
        );
        let mut live = Live::start_with(fake(&script), Arc::new(Mutex::new(Vec::new()))).unwrap();
        let done = live
            .finish_within(&wav(&[0.1; 16000]), Duration::from_secs(2))
            .unwrap();
        assert_eq!(done.transcript.text, "First. Tail.");
        assert_eq!(
            done.transcript
                .words
                .iter()
                .map(|w| (w.t0, w.t1))
                .collect::<Vec<_>>(),
            vec![(63, 150), (913, 1000)]
        );
    }

    #[cfg(unix)]
    #[test]
    fn failure_after_a_completed_prefix_returns_no_partial_transcript() {
        let script = format!(
            r#"{READY}; cat >/dev/null; echo '{{"segment":{{"t0":0,"t1":100,"text":"Private prefix."}},"words":[{{"t0":0,"t1":100,"text":"Private prefix."}}]}}'; echo 'apparent-helper: injected tail failure' >&2; exit 1"#
        );
        let mut live = Live::start_with(fake(&script), Arc::new(Mutex::new(Vec::new()))).unwrap();
        let result = live.finish_within(&wav(&[0.1; 16000]), Duration::from_secs(2));
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("injected tail failure"));
    }

    #[cfg(unix)]
    #[test]
    fn reports_the_selected_apple_module() {
        for engine in ["SpeechTranscriber", "DictationTranscriber"] {
            let script = format!(
                r#"echo '{{"ready":true,"locale":"en_US","engine":"{engine}"}}'; cat >/dev/null; echo '{{"segment":{{"t0":0,"t1":20,"text":"Tail."}},"words":[{{"t0":0,"t1":20,"text":"Tail."}}]}}'; echo '{{"done":true,"samples":320}}'"#
            );
            let mut live =
                Live::start_with(fake(&script), Arc::new(Mutex::new(Vec::new()))).unwrap();
            let done = live
                .finish_within(&wav(&[0.1; 320]), Duration::from_secs(2))
                .unwrap();
            assert_eq!(done.transcript.model, Some(format!("{engine} en_US")));
            assert_eq!(done.transcript.text, "Tail.");
            assert_eq!(done.transcript.words.len(), 1);
            assert_eq!(done.transcript.words[0].t1, 20);
        }
    }

    /// A stand-in helper: echoes how many samples it read, as the real one does, after the
    /// given lines.
    fn fake(script: &str) -> Command {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        command
    }

    #[cfg(unix)]
    fn wav(samples: &[f32]) -> Vec<u8> {
        crate::audio::encode_wav_16k(samples).unwrap()
    }

    #[cfg(unix)]
    const READY: &str = r#"echo '{"ready":true,"locale":"en_US"}'"#;

    #[cfg(unix)]
    #[test]
    fn collects_finalized_results_and_counts_every_sample() {
        let script = format!(
            r#"{READY}; n=$(wc -c | tr -d ' '); echo '{{"segment":{{"t0":900,"t1":1500,"text":"Look here."}},"words":[{{"t0":900,"t1":1200,"text":"Look"}},{{"t0":1200,"t1":1500,"text":"here."}}]}}'; echo '{{"segment":{{"t0":60,"t1":600,"text":"Okay,"}},"words":[{{"t0":60,"t1":600,"text":"Okay,"}}]}}'; echo "{{\"done\":true,\"samples\":$((n / 2))}}""#
        );
        let buffer = Arc::new(Mutex::new(vec![0.25_f32; 4_000]));
        let mut live = Live::start_with(fake(&script), Arc::clone(&buffer)).unwrap();
        std::thread::sleep(Duration::from_millis(250));
        let mut all = vec![0.25_f32; 4_000];
        all.extend([-0.5_f32; 28_000]);
        let done = live
            .finish_within(&wav(&all), Duration::from_secs(5))
            .unwrap();
        assert_eq!(done.streamed_samples, 4_000);
        assert_eq!(done.transcript.text, "Okay, Look here.");
        assert_eq!(done.transcript.source, "speech-analyzer");
        assert_eq!(
            done.transcript.model.as_deref(),
            Some("SpeechTranscriber en_US")
        );
        assert_eq!(
            done.transcript
                .words
                .iter()
                .map(|w| (w.t0, w.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(60, "Okay,"), (900, "Look"), (1200, "here.")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn streams_exactly_the_samples_the_wav_holds() {
        let out = std::env::temp_dir().join(format!("apparent-live-pcm-{}", std::process::id()));
        let script = format!(
            r#"{READY}; cat > '{}'; echo '{{"done":true,"samples":3}}'"#,
            out.display()
        );
        let buffer = Arc::new(Mutex::new(vec![0.5_f32, -0.25]));
        let mut live = Live::start_with(fake(&script), Arc::clone(&buffer)).unwrap();
        std::thread::sleep(Duration::from_millis(250));
        buffer.lock().unwrap().clear();
        let recording = [0.5_f32, -0.25, 1.5];
        live.finish_within(&wav(&recording), Duration::from_secs(5))
            .unwrap();
        let written = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        let expected = crate::audio::decode_wav_pcm16(&wav(&recording)).unwrap();
        assert_eq!(written, le_bytes(&expected));
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_helper_or_lost_samples_is_an_error() {
        let buffer = || Arc::new(Mutex::new(vec![0.1_f32; 320]));
        let failed = Live::start_with(
            fake("echo 'apparent-helper: the speech model isnt installed' >&2; exit 1"),
            buffer(),
        )
        .unwrap()
        .finish_within(&wav(&[0.1; 320]), Duration::from_secs(5));
        assert!(failed.is_err());

        let short = format!(r#"{READY}; cat > /dev/null; echo '{{"done":true,"samples":7}}'"#);
        let lost = Live::start_with(fake(&short), buffer())
            .unwrap()
            .finish_within(&wav(&[0.1; 320]), Duration::from_secs(5));
        assert!(lost.unwrap_err().to_string().contains("heard 7 samples"));

        let hung = Live::start_with(
            fake(&format!("{READY}; cat > /dev/null; sleep 30")),
            buffer(),
        )
        .unwrap()
        .finish_within(&wav(&[0.1; 320]), Duration::from_millis(300));
        assert!(hung.unwrap_err().to_string().contains("to finish"));
    }

    #[test]
    fn dropping_an_unfinished_stream_stops_the_helper() {
        let live = Live::start_with(fake("sleep 30"), Arc::new(Mutex::new(Vec::new()))).unwrap();
        let started = Instant::now();
        drop(live);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn finish_deadline_includes_a_blocked_feeder_and_tail_write() {
        for already_streaming in [false, true] {
            let samples = vec![0.25; 160_000];
            let buffer = Arc::new(Mutex::new(if already_streaming {
                samples.clone()
            } else {
                Vec::new()
            }));
            // No stdin reads: both a recording-time write and the WAV tail can fill the pipe.
            // A finite stand-in bounds the regression itself on the old implementation.
            let mut live = Live::start_with(fake("exec sleep 2"), buffer).unwrap();
            std::thread::sleep(Duration::from_millis(100));
            let started = Instant::now();
            assert!(
                live.finish_within(&wav(&samples), Duration::from_millis(100))
                    .is_err()
            );
            assert!(
                started.elapsed() < Duration::from_millis(700),
                "blocked Finish: {:?}",
                started.elapsed()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_interrupts_a_blocked_recording_write() {
        let buffer = Arc::new(Mutex::new(vec![0.25; 160_000]));
        let live = Live::start_with(fake("exec sleep 2"), buffer).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let started = Instant::now();
        drop(live);
        assert!(
            started.elapsed() < Duration::from_millis(700),
            "blocked Cancel: {:?}",
            started.elapsed()
        );
    }

    #[cfg(unix)]
    #[test]
    fn corrupt_streams_cannot_replace_whole_wav_transcription() {
        for events in [
            r#"echo '{"ready":false,"locale":"en_US"}'; echo '{"done":true,"samples":16000}'"#
                .to_owned(),
            format!(r#"{READY}; echo '{{"done":false,"samples":16000}}'"#),
            format!(r#"{READY}; echo 'not json'; echo '{{"done":true,"samples":16000}}'"#),
            format!(
                r#"{READY}; echo '{{"segment":{{"t0":0,"t1":1001,"text":"beyond end"}}}}'; echo '{{"done":true,"samples":16000}}'"#
            ),
            format!(
                r#"{READY}; echo '{{"segment":{{"t0":800,"t1":100,"text":"backwards"}}}}'; echo '{{"done":true,"samples":16000}}'"#
            ),
            format!(
                r#"{READY}; echo '{{"segment":{{"t0":0,"t1":100,"text":"word"}},"words":[{{"t0":0,"t1":1001,"text":"word"}}]}}'; echo '{{"done":true,"samples":16000}}'"#
            ),
            r#"echo '{"ready":true,"locale":"en_US","engine":"Unknown"}'; echo '{"done":true,"samples":16000}'"#.to_owned(),
        ] {
            let mut live = Live::start_with(
                fake(&format!("cat >/dev/null; {events}")),
                Arc::new(Mutex::new(Vec::new())),
            )
            .unwrap();
            assert!(
                live.finish_within(&wav(&vec![0.1; 16000]), Duration::from_secs(2))
                    .is_err(),
                "accepted {events}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn stopped_mid_write_resends_only_the_unwritten_wav_tail() {
        let dir = std::env::temp_dir().join(format!("apparent-live-tail-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let out = dir.join("pcm");
        let script = format!(
            r#"{READY}; sleep 0.2; cat > '{}'; echo '{{"done":true,"samples":160000}}'"#,
            out.display()
        );
        let samples: Vec<f32> = (0..160_000).map(|i| (i % 1000) as f32 / 1000.0).collect();
        let mut live =
            Live::start_with(fake(&script), Arc::new(Mutex::new(samples.clone()))).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let recording = wav(&samples);
        live.finish_within(&recording, Duration::from_secs(3))
            .unwrap();
        let actual = std::fs::read(&out).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(
            actual,
            le_bytes(&crate::audio::decode_wav_pcm16(&recording).unwrap())
        );
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_does_not_wait_for_the_microphone_buffer_lock() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let guard = buffer.lock().unwrap();
        let live = Live::start_with(fake("exec sleep 2"), Arc::clone(&buffer)).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let started = Instant::now();
        drop(live);
        assert!(started.elapsed() < Duration::from_millis(700));
        drop(guard);
    }

    #[cfg(unix)]
    #[test]
    fn helper_stderr_cannot_block_audio_or_completion() {
        let script = format!(
            r#"{READY}; dd if=/dev/zero bs=1024 count=256 1>&2 2>/dev/null; n=$(wc -c | tr -d ' '); echo "{{\"done\":true,\"samples\":$((n / 2))}}""#
        );
        let mut live = Live::start_with(fake(&script), Arc::new(Mutex::new(Vec::new()))).unwrap();
        live.finish_within(&wav(&[0.1; 320]), Duration::from_secs(2))
            .unwrap();
    }
}

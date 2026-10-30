//! Explicit two-pass agent workflow; ordinary captures and exports are unchanged.
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Subcommand)]
pub enum WalkthroughCommand {
    /// Record the real task using the existing timed recorder (no click/ink telemetry).
    /// For interactive telemetry use the app shortcut, then inspect its exact saved bundle.
    Record {
        #[arg(long)]
        duration: u64,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        window: Option<String>,
        /// A factual capture note, not an inferred pass/fail result.
        #[arg(long)]
        note: Option<String>,
    },
    /// Read saved evidence, movie-clock frame/event times and a blank editable cue template.
    /// Missing manifests report finalizing_or_incomplete, never ready.
    Inspect {
        bundle: PathBuf,
        /// Write a NEW cue file. Fill its empty text after inspecting the movie and images.
        #[arg(long)]
        template: Option<PathBuf>,
    },
    /// Render a separate narrated/captioned share copy; original time unless explicitly compressed.
    Export {
        bundle: PathBuf,
        /// Opt into the off-by-default rendering experiment for this process only.
        #[arg(long, value_parser = ["agentWalkthrough"])]
        experiment: Option<String>,
        #[arg(long)]
        cues: PathBuf,
        /// A new private directory outside the source bundle; refuses overwrites.
        #[arg(long)]
        out_dir: PathBuf,
        /// Explicit privacy choice: keep reviewed microphone/movie audio, or drop it.
        #[arg(long, value_parser = ["keep", "drop"])]
        source_audio: String,
        /// Consent to send cue TEXT ONLY to Microsoft Edge TTS, never footage or source audio.
        #[arg(long)]
        allow_online_tts: bool,
        #[arg(long)]
        no_voice: bool,
        #[arg(long)]
        no_captions: bool,
        /// Speed gaps outside measured speech/readable captions. Requires source-audio drop.
        #[arg(long, conflicts_with = "no_compression")]
        compress_gaps: bool,
        /// Keep original time (the default).
        #[arg(long)]
        no_compression: bool,
        #[arg(long, default_value_t = 4.0)]
        gap_rate: f64,
        #[arg(long, default_value_t = 0.25)]
        gap_padding: f64,
        /// Source seconds START,END to preserve at real time; repeat as needed.
        #[arg(long)]
        protect_range: Vec<String>,
        /// Reuse reviewed cue-NNN.mp3 files and matching cues.json without network TTS.
        #[arg(long)]
        speech_dir: Option<PathBuf>,
        /// No audio at all. Implies no voice and requires source-audio drop.
        #[arg(long)]
        silent: bool,
        #[arg(long = "no-zoom", action = clap::ArgAction::SetFalse, default_value_t = true)]
        zoom: bool,
        #[arg(long, default_value = "full")]
        crop: crate::export::crop::CropArg,
        /// Regular sans-serif TTF. If absent, check standard Linux/macOS font locations.
        #[arg(long)]
        font: Option<PathBuf>,
        /// Python 3.11+ with Pillow==11.3.0 and edge-tts==7.2.3 installed. No auto-install.
        #[arg(long, default_value = "python3")]
        python: PathBuf,
    },
}

fn checked_media(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = dir.join(name).canonicalize()?;
    ensure!(
        path.starts_with(dir),
        "bundle media must stay inside its bundle"
    );
    ensure!(path.is_file(), "bundle media is not a file");
    Ok(path)
}

fn movie_info(movie: &Path) -> Result<Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(movie)
        .output()
        .context("walkthrough needs ffprobe on PATH (install FFmpeg separately)")?;
    ensure!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub fn inspect(bundle: &Path, template: Option<&Path>) -> Result<Value> {
    let dir = bundle
        .canonicalize()
        .context("bundle directory not found; use the exact saved path")?;
    if !dir.join("apparent.json").is_file() {
        return Ok(json!({"status":"finalizing_or_incomplete", "bundle":dir,
            "instructions":"Wait for the capture to finish and apparent.json to appear. If processing ended without it, inspect capture errors; do not narrate an unfinished take."}));
    }
    let (_, loaded) = crate::handoff::load_bundle(&dir)?;
    let movie = checked_media(
        &dir,
        loaded
            .video
            .as_deref()
            .context("bundle has no movie; record a real task first")?,
    )?;
    let info = movie_info(&movie)?;
    let duration = info["streams"]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["codec_type"] == "video"))
        .and_then(|s| s["duration"].as_str())
        .or_else(|| info["format"]["duration"].as_str())
        .context("movie has no duration")?
        .parse::<f64>()?;
    ensure!(
        duration.is_finite() && duration > 0.0,
        "movie has no usable duration"
    );
    let cue_template = json!([[0.0, duration, ""]]);
    if let Some(path) = template {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .context("choose a new cue-template path; existing text is never overwritten")?;
        file.write_all(serde_json::to_string_pretty(&cue_template)?.as_bytes())?;
        file.write_all(b"\n")?;
    }
    let frames: Vec<Value> = loaded.keyframes.iter().map(|frame| {
        let value = serde_json::to_value(frame).expect("keyframe serializes");
        json!({"evidence":value, "path":dir.join(&frame.file), "movieSeconds": (frame.t as f64 - loaded.video_offset_ms as f64)/1000.0})
    }).collect();
    Ok(
        json!({"status":"ready", "bundle":dir, "movie":movie, "durationSeconds":duration,
        "manifest":dir.join("apparent.json"), "summary":dir.join("summary.md"),
        "contactSheet":dir.join(&loaded.contact_sheet), "framesDirectory":dir.join("frames"),
        "videoOffsetMs":loaded.video_offset_ms, "frames":frames, "events":loaded.events,
        "marks":loaded.marks, "note":loaded.note, "interactionTelemetry":loaded.interaction_telemetry,
        "cueClock":"movie seconds; movieSeconds = (bundle milliseconds - videoOffsetMs) / 1000; negative moments precede the movie",
        "cueTemplate":cue_template,
        "instructions":"Inspect summary, frames/events AND the entire saved movie. Replace empty text with short natural explanatory sentences supported by visible evidence, split into nonoverlapping windows within durationSeconds. Empty text is deliberately invalid. Use apparent frames/zoom for closer inspection. Never infer a pass/fail claim from a note. Export again to a new out-dir after editing cues; do not re-record just to revise text."}),
    )
}

pub fn run(command: WalkthroughCommand) -> Result<Value> {
    match command {
        WalkthroughCommand::Inspect { bundle, template } => inspect(&bundle, template.as_deref()),
        WalkthroughCommand::Record {
            duration,
            out,
            window,
            note,
        } => {
            ensure!(
                !out.exists(),
                "choose a new bundle path; refusing existing recording"
            );
            let mut child = Command::new(std::env::current_exe()?);
            child
                .arg("record")
                .arg("--duration")
                .arg(duration.to_string())
                .arg("--out")
                .arg(&out);
            if let Some(window) = window {
                child.arg("--window").arg(window);
            }
            if let Some(note) = note {
                child.arg("--note").arg(note);
            }
            // Keep stdout machine-readable; ordinary recorder's summary goes to stderr.
            child.stdout(Stdio::from(std::io::stderr()));
            ensure!(
                child.status()?.success(),
                "recording failed; inspect capture errors before retrying"
            );
            inspect(&out, None)
        }
        WalkthroughCommand::Export {
            bundle,
            experiment,
            cues,
            out_dir,
            source_audio,
            allow_online_tts,
            no_voice,
            no_captions,
            compress_gaps,
            no_compression,
            gap_rate,
            gap_padding,
            protect_range,
            speech_dir,
            silent,
            zoom,
            crop,
            font,
            python,
        } => {
            if let Some(name) = experiment {
                crate::handoff::enable_experiment(&name);
            }
            ensure!(crate::handoff::experiment("agentWalkthrough"), "agent walkthrough rendering is experimental: pass --experiment agentWalkthrough, or enable experiments.agentWalkthrough in config.json. Ordinary capture/export defaults are unchanged.");
            ensure!(
                !silent || source_audio == "drop",
                "--silent requires --source-audio drop"
            );
            ensure!(
                no_voice || silent || allow_online_tts || speech_dir.is_some(),
                "voice is on by default: --allow-online-tts sends cue text only, or use --no-voice/--silent"
            );
            let evidence = inspect(&bundle, None)?;
            ensure!(
                evidence["status"] == "ready",
                "capture is finalizing or incomplete; run walkthrough inspect again"
            );
            let dir = bundle.canonicalize()?;
            let (_, loaded) = crate::handoff::load_bundle(&dir)?;
            if let Some(audio) = &loaded.audio {
                checked_media(&dir, audio)?;
            }
            let parent = out_dir
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()?;
            let out_dir = parent.join(
                out_dir
                    .file_name()
                    .context("out-dir needs a new directory name")?,
            );
            ensure!(
                !out_dir.starts_with(&dir),
                "walkthrough output must be outside the source bundle"
            );
            ensure!(
                !out_dir.exists(),
                "out-dir already exists; choose a new render directory"
            );
            let font = font
                .or_else(|| {
                    [
                        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                        "/usr/share/fonts/TTF/DejaVuSans.ttf",
                        "/System/Library/Fonts/Supplemental/Arial.ttf",
                    ]
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.is_file())
                })
                .context(
                    "regular sans-serif TTF unavailable; pass --font /absolute/path/font.ttf",
                )?;
            // The renderer is embedded in installed binaries, not located relative to a checkout.
            // It runs the existing export command in its private workspace before postproduction.
            let mut child = Command::new(python);
            child
                // Do not import executable modules from a capture/current directory or PYTHONPATH.
                .arg("-I")
                .arg("-c")
                .arg(include_str!("../scripts/proof-video.py"))
                .arg(evidence["movie"].as_str().context("movie path missing")?)
                .arg("--cues")
                .arg(cues)
                .arg("--out-dir")
                .arg(&out_dir)
                .arg("--source-audio")
                .arg(source_audio)
                .arg("--font")
                .arg(font)
                .arg("--bundle")
                .arg(&dir)
                .arg("--apparent")
                .arg(std::env::current_exe()?)
                .arg("--bundle-crop")
                .arg(match crop {
                    crate::export::crop::CropArg::Full => "full".to_owned(),
                    crate::export::crop::CropArg::Auto => "auto".to_owned(),
                    crate::export::crop::CropArg::Rect(rect) => rect
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                });
            if !zoom {
                child.arg("--no-zoom");
            }
            if compress_gaps {
                child.arg("--compress-gaps");
            }
            if no_compression {
                child.arg("--no-compression");
            }
            child.arg("--gap-rate").arg(gap_rate.to_string());
            child.arg("--gap-padding").arg(gap_padding.to_string());
            for range in protect_range {
                child.arg("--protect-range").arg(range);
            }
            if let Some(path) = speech_dir {
                child.arg("--speech-dir").arg(path);
            }
            if allow_online_tts {
                child.arg("--allow-online-tts");
            }
            if no_voice {
                child.arg("--no-voice");
            }
            if no_captions {
                child.arg("--no-captions");
            }
            if silent {
                child.arg("--silent");
            }
            let result = child
                .output()
                .context("Python 3.11+ unavailable; pass --python /path/to/prepared/python")?;
            ensure!(
                result.status.success(),
                "walkthrough renderer failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            ensure!(
                out_dir.join("share.mp4").is_file(),
                "renderer did not publish share.mp4"
            );
            Ok(
                json!({"status":"ready", "bundle":dir, "output":out_dir.join("share.mp4"),
                "provenance":out_dir.join("provenance.json"), "timing":out_dir.join("timing-report.json"),
                "cues":out_dir.join("cues.json"), "qa":out_dir.join("qa.json")}),
            )
        }
    }
}

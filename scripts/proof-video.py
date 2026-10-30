#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==11.3.0", "edge-tts==7.2.3"]
# ///
"""Private agent proof-video postproduction; never edits an Apparent bundle."""
import argparse
import array
import asyncio
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import wave
import os
import shutil


def timing_map(duration, realtime, rate=4.0, padding=0.25):
    """Continuous source→output map; protected spans and narration stay real time."""
    if not math.isfinite(duration) or duration <= 0:
        raise ValueError('duration must be finite and positive')
    if not math.isfinite(rate) or not 1 <= rate <= 16:
        raise ValueError('gap rate must be finite, between 1 and 16')
    if not math.isfinite(padding) or not 0 <= padding <= 5:
        raise ValueError('gap padding must be finite, between 0 and 5 seconds')
    spans = []
    for a, b in sorted(realtime):
        if not all(math.isfinite(v) for v in (a, b)) or not 0 <= a < b <= duration:
            raise ValueError('protected ranges must be finite, ordered and within source duration')
        a, b = max(0, a-padding), min(duration, b+padding)
        if spans and a <= spans[-1][1]:
            spans[-1][1] = max(b, spans[-1][1])
        else:
            spans.append([a, b])
    points = sorted({0.0, duration, *(v for span in spans for v in span)})
    result, output = [], 0.0
    for a, b in zip(points, points[1:]):
        speed = 1.0 if any(x <= a and b <= y for x, y in spans) else rate
        end = output + (b-a)/speed
        if result and result[-1]['rate'] == speed:
            result[-1].update(source_end=b, output_end=end)
        else:
            result.append(dict(source_start=a, source_end=b, output_start=output, output_end=end, rate=speed))
        output = end
    return result


def mapped(t, segments):
    for s in segments:
        if s['source_start'] <= t <= s['source_end']:
            return s['output_start'] + (t-s['source_start'])/s['rate']
    raise ValueError('time outside source map')


def protected_range(text):
    try:
        a, b = map(float, text.split(','))
        if not all(math.isfinite(v) for v in (a, b)) or not 0 <= a < b:
            raise ValueError()
        return a, b
    except ValueError:
        raise argparse.ArgumentTypeError('expected finite source seconds START,END with 0 <= START < END')


def balanced_sum(terms):
    """Bound FFmpeg expression depth logarithmically, not one nested if per span."""
    if not terms:
        return '0'
    if len(terms) == 1:
        return terms[0]
    middle = len(terms)//2
    return f'({balanced_sum(terms[:middle])}+{balanced_sum(terms[middle:])})'


def validate(cues, duration, crop, width, height):
    if not isinstance(cues, list) or not cues:
        raise ValueError("cues must be a nonempty array of [start, end, text]")
    previous = 0
    for cue in cues:
        if not isinstance(cue, list) or len(cue) != 3:
            raise ValueError("each cue must be [start, end, text]")
        a, b, text = cue
        if any(type(t) not in (int, float) or not math.isfinite(t) for t in (a, b)):
            raise ValueError("cue times must be finite numbers")
        if not previous <= a < b <= duration + 0.001:
            raise ValueError("cues overlap, are unordered, or exceed source duration")
        if not isinstance(text, str) or not text.strip() or len(text.splitlines()) > 2:
            raise ValueError("cue text must contain one or two nonempty spoken lines")
        previous = b
    if crop is not None:
        x, y, w, h = crop
        if any(type(n) is not int for n in crop) or min(x, y) < 0 or min(w, h) < 2 or x + w > width or y + h > height:
            raise ValueError("crop x,y,w,h must be inside source, with positive dimensions")
    return cues


def run(*args):
    return subprocess.check_output(list(map(str, args)))


def probe(path):
    return json.loads(run('ffprobe', '-v', 'error', '-show_streams', '-show_format', '-of', 'json', path))


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def evidence_hashes(bundle):
    """Pin all original bundle files, not just the exported intermediate movie."""
    if bundle is None:
        return {}
    bundle = bundle.resolve(strict=True)
    result = {}
    for path in sorted(bundle.rglob('*')):
        if path.is_symlink():
            raise ValueError('source bundle must not contain symlinks')
        if path.is_file():
            result[str(path)] = digest(path)
    return result


def rect(text):
    try:
        values = tuple(int(v) for v in text.split(','))
        if len(values) != 4:
            raise ValueError()
        return values
    except ValueError:
        raise argparse.ArgumentTypeError('expected x,y,width,height integers')


def stamp(t):
    ms = round(t * 1000)
    hours, ms = divmod(ms, 3600000)
    minutes, ms = divmod(ms, 60000)
    seconds, ms = divmod(ms, 1000)
    return f'{hours:02}:{minutes:02}:{seconds:02},{ms:03}'


def fit_speech(seconds, start, end):
    if seconds > end - start - 0.10:
        raise ValueError(f'speech needs {seconds:.2f}s, window {start:g}–{end:g}s: shorten text; no time stretching')


def qa(path, duration, audio):
    data = probe(path)
    video = next(s for s in data['streams'] if s['codec_type'] == 'video')
    audios = [s for s in data['streams'] if s['codec_type'] == 'audio']
    if video['codec_name'] != 'h264' or video['pix_fmt'] != 'yuv420p' or video['r_frame_rate'] != '30/1':
        raise ValueError('export failed H264/YUV420p/30fps QA')
    if abs(float(data['format']['duration']) - duration) > 0.10:
        raise ValueError('export changed planned duration')
    if bool(audios) != audio or any(s['codec_name'] != 'aac' for s in audios):
        raise ValueError('export failed audio-policy QA')
    run('ffmpeg', '-v', 'error', '-xerror', '-i', path, '-f', 'null', '-')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--cues', type=Path, required=True)
    parser.add_argument('--out-dir', type=Path, required=True, help='new private directory; refuses existing paths')
    parser.add_argument('--source-audio', choices=['drop', 'keep'], required=True, help='explicit privacy decision; keep mixes source into voice-over')
    parser.add_argument('--silent', action='store_true', help='captions only, no TTS; requires source-audio drop')
    parser.add_argument('--no-voice', action='store_true', help='omit TTS independently of source audio and captions')
    parser.add_argument('--no-captions', action='store_true', help='omit burned explanatory captions independently of voice')
    compression = parser.add_mutually_exclusive_group()
    compression.add_argument('--compress-gaps', action='store_true', help='speed gaps outside measured narration/readable captions; requires source-audio drop')
    compression.add_argument('--no-compression', action='store_true', help='preserve original time (default)')
    parser.add_argument('--gap-rate', type=float, default=4.0)
    parser.add_argument('--gap-padding', type=float, default=0.25)
    parser.add_argument('--protect-range', type=protected_range, action='append', default=[], help='repeatable source seconds START,END kept at real time')
    parser.add_argument('--speech-dir', type=Path, help='reuse reviewed cue-NNN.mp3 clips from a prior render; no network')
    parser.add_argument('--bundle', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--apparent', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--bundle-crop', default='full', help=argparse.SUPPRESS)
    parser.add_argument('--no-zoom', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--allow-online-tts', action='store_true', help='consent to send cue text ONLY to Microsoft Edge TTS')
    parser.add_argument('--voice', default='en-GB-RyanNeural')
    parser.add_argument('--rate', default='+10%')
    parser.add_argument('--crop', type=rect, help='source x,y,width,height; static measured close-up, no retiming')
    parser.add_argument('--caption-box', type=rect, default=(40, 730, 1360, 140), help='safe output x,y,width,height; keep results outside this region')
    parser.add_argument('--font', type=Path, required=True, help='regular sans-serif TTF, e.g. DejaVuSans.ttf')
    args = parser.parse_args()
    if args.silent and args.source_audio != 'drop':
        raise ValueError('--silent requires --source-audio drop')
    voice = not (args.no_voice or args.silent)
    if args.compress_gaps and args.source_audio != 'drop':
        raise ValueError('--compress-gaps requires --source-audio drop; retained source audio must stay at original time')
    if args.speech_dir and not voice:
        raise ValueError('--speech-dir requires voice')
    if voice and not args.allow_online_tts and not args.speech_dir:
        raise ValueError('TTS sends cue text externally: pass --allow-online-tts or --no-voice/--silent')
    if sys.version_info < (3, 11):
        raise ValueError('Python 3.11+ required; select a prepared interpreter with --python')
    try:
        from PIL import Image, ImageDraw, ImageFont
        tts_version = None
        if voice and not args.speech_dir:
            import edge_tts
            tts_version = getattr(edge_tts, '__version__', 'fixture/unreported')
    except ImportError as exc:
        raise ValueError('prepare a Python environment with Pillow==11.3.0 and edge-tts==7.2.3; pass its python using --python. Nothing is auto-installed.') from exc
    original_hashes = evidence_hashes(args.bundle)
    source = args.source.resolve(strict=True)
    source_hash = digest(source)
    info = probe(source)
    video = next(s for s in info['streams'] if s['codec_type'] == 'video')
    duration = float(video.get('duration', info['format']['duration']))
    timing_map(duration, args.protect_range, args.gap_rate, args.gap_padding)
    cue_bytes = args.cues.read_bytes()
    cues = validate(json.loads(cue_bytes), duration, args.crop, int(video['width']), int(video['height']))
    x, y, w, h = args.caption_box
    if min(x, y) < 0 or min(w, h) < 40 or x + w > 1440 or y + h > 900:
        raise ValueError('caption box must fit 1440x900 output')
    font = ImageFont.truetype(str(args.font), 32)
    # Validate all captions before creating output or sending text to TTS.
    images = []
    for _, _, text in cues:
        image = Image.new('RGBA', (1440, 900))
        draw = ImageDraw.Draw(image)
        bb = draw.multiline_textbbox((0, 0), text, font=font, spacing=7, align='center')
        tw, th = bb[2] - bb[0], bb[3] - bb[1]
        if not args.no_captions and (tw + 30 > w or th + 30 > h):
            raise ValueError('caption exceeds safe box: shorten or insert one line break')
        left, top = x + (w - tw) // 2, y + h - 15 - th
        draw.rounded_rectangle((left-15, top-15, left+tw+15, top+th+15), radius=8, fill=(10, 10, 10, 220))
        draw.multiline_text((left-bb[0], top-bb[1]), text, font=font, fill='white', spacing=7, align='center')
        if args.no_captions:
            image = Image.new('RGBA', (1440, 900))
        images.append(image)
    out = args.out_dir.resolve()
    if args.bundle and out.is_relative_to(args.bundle.resolve()):
        raise ValueError('output must be outside the source bundle')
    out.mkdir(mode=0o700, parents=False, exist_ok=False)
    raw_source = source
    if args.bundle:
        if args.apparent is None:
            raise ValueError('bundle workflow needs the installed Apparent executable')
        command = [args.apparent, 'export', args.bundle, '--out', out / 'camera.mp4', '--fps', '30', '--crop', args.bundle_crop]
        if args.no_zoom:
            command.append('--no-zoom')
        run(*command)
        source = out / 'camera.mp4'
        info = probe(source)
        video = next(s for s in info['streams'] if s['codec_type'] == 'video')
        if abs(float(video.get('duration', info['format']['duration'])) - duration) > 0.10:
            raise ValueError('camera export changed real-time duration')
    (out / 'cues.json').write_text(json.dumps(cues, indent=2) + '\n')
    (out / 'spoken-script.txt').write_text('\n\n'.join(t.replace('\n', ' ') for _, _, t in cues) + '\n')

    policy = {'source': str(raw_source), 'source_sha256': source_hash, 'evidence_sha256': original_hashes, 'source_audio': args.source_audio, 'captions': not args.no_captions, 'synthetic_narration': voice, 'voice': args.voice if voice else None, 'rate': args.rate, 'crop': args.crop, 'bundle_crop': args.bundle_crop, 'zoom': not args.no_zoom, 'caption_box': args.caption_box, 'duration': duration}
    import PIL
    policy.update({'cue_input_sha256': hashlib.sha256(cue_bytes).hexdigest(), 'font_sha256': digest(args.font),
                   'apparent_sha256': digest(args.apparent) if args.apparent else None,
                   'python': sys.version, 'pillow': PIL.__version__,
                   'edge_tts': tts_version})
    timing = []
    clips = []
    clip_hashes = {}
    if voice:
        async def synthesize():
            for i, (_, _, text) in enumerate(cues):
                await edge_tts.Communicate(text.replace('\n', ' '), voice=args.voice, rate=args.rate).save(str(out / f'cue-{i:03}.mp3'))
        async def bounded_synthesis():
            await asyncio.wait_for(synthesize(), timeout=180)
        if args.speech_dir:
            if json.loads((args.speech_dir / 'cues.json').read_text()) != cues:
                raise ValueError('reused speech cues differ; use matching reviewed clips or synthesize new speech')
            for i in range(len(cues)):
                path = args.speech_dir / f'cue-{i:03}.mp3'
                shutil.copyfile(path, out / path.name)
        else:
            try:
                asyncio.run(bounded_synthesis())
            except Exception as exc:
                # Provider exceptions can embed service URLs/tokens; disclose the failure, not those.
                status = getattr(exc, 'status', None)
                reason = f'HTTP {status}' if status else type(exc).__name__
                raise ValueError(f'online TTS failed ({reason}); no share copy published. Retry later into a NEW out-dir, or explicitly use --no-voice/--silent. No audio fallback was substituted.') from None
        sample_rate = 48000

        for i, (a, b, text) in enumerate(cues):
            raw = run('ffmpeg', '-v', 'error', '-i', out / f'cue-{i:03}.mp3', '-ar', sample_rate, '-ac', 1, '-f', 's16le', '-')
            samples = array.array('h')
            samples.frombytes(raw)
            seconds = len(samples) / sample_rate
            fit_speech(seconds, a, b)
            clips.append(samples)
            clip_hashes[f'cue-{i:03}.mp3'] = digest(out / f'cue-{i:03}.mp3')
            timing.append({'cue': i, 'start': a+0.05, 'end': a+0.05+seconds, 'text': text, 'tempo': 1})
    caption_cues = []
    for i, (a, b, text) in enumerate(cues):
        # Preserve validate()'s existing 1 ms rounded-end tolerance, while the
        # map itself remains strictly inside the actual source duration.
        end = min(b, duration)
        if args.compress_gaps and voice and not args.no_captions:
            # Readability is independent of how fast the generated voice speaks.
            readable = max(1.0, len(text.replace('\n', ' '))/15)
            if readable > b-a:
                raise ValueError('caption window too short to read at 15 characters/s; extend it or use --no-captions')
            end = max(a+readable, timing[i]['end'])
        caption_cues.append([a, end, text])
    realtime = list(args.protect_range) + [(t['start'], t['end']) for t in timing]
    if not args.no_captions:
        realtime += [(a, b) for a, b, _ in caption_cues]
    segments = timing_map(duration, realtime, args.gap_rate if args.compress_gaps else 1.0, args.gap_padding)
    output_duration = segments[-1]['output_end']
    output_cues = [[mapped(a, segments), mapped(b, segments), text] for a, b, text in caption_cues]
    (out / 'explanation.srt').write_text('' if args.no_captions else '\n'.join(f'{i+1}\n{stamp(a)} --> {stamp(b)}\n{t}\n' for i, (a, b, t) in enumerate(output_cues)))
    timeline = ['ffconcat version 1.0']
    Image.new('RGBA', (1440, 900)).save(out / 'blank.png')
    previous = 0
    for i, ((a, b, _), image) in enumerate(zip(output_cues, images)):
        if a > previous:
            timeline += ["file 'blank.png'", f'duration {a-previous:.9f}']
        image.save(out / f'caption-{i:03}.png')
        timeline += [f"file 'caption-{i:03}.png'", f'duration {b-a:.9f}']
        previous = b
    if output_duration > previous:
        timeline += ["file 'blank.png'", f'duration {output_duration-previous:.9f}']
    timeline += ["file 'blank.png'"]
    (out / 'captions.ffconcat').write_text('\n'.join(timeline) + '\n')
    if voice:
        mix = array.array('h', [0]) * round(output_duration * sample_rate)
        for t, samples in zip(timing, clips):
            t['source_start'], t['source_end'] = t['start'], t['end']
            t['start'], t['end'] = mapped(t['start'], segments), mapped(t['end'], segments)
            start = round(t['start'] * sample_rate)
            mix[start:start+len(samples)] = samples
        if sys.byteorder != 'little':
            mix.byteswap()
        with wave.open(str(out / 'narration.wav'), 'wb') as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(sample_rate)
            wav.writeframes(mix.tobytes())
    (out / 'timing-report.json').write_text(json.dumps(timing, indent=2) + '\n')
    policy.update({'compress_gaps': args.compress_gaps, 'gap_rate': args.gap_rate, 'gap_padding': args.gap_padding,
                   'protected_ranges': args.protect_range, 'time_map': segments, 'output_duration': output_duration,
                   'output_cues': output_cues if not args.no_captions else [], 'speech_sha256': clip_hashes,
                   'speech_reused_from': str(args.speech_dir.resolve()) if args.speech_dir else None})
    (out / 'provenance.json').write_text(json.dumps(policy, indent=2) + '\n')
    # Visible disclosure survives detachment from this script and its sidecars.
    badge = Image.new('RGBA', (1440, 900))
    draw = ImageDraw.Draw(badge)
    label = ('Synthetic voice-over' if voice else 'Agent walkthrough') + ' · source audio ' + args.source_audio
    small = ImageFont.truetype(str(args.font), 20)
    draw.rounded_rectangle((12, 12, 620, 48), radius=8, fill=(10, 10, 10, 220))
    draw.text((24, 18), label, font=small, fill='white')
    badge.save(out / 'disclosure.png')
    command = ['ffmpeg', '-v', 'error', '-nostdin', '-i', str(source), '-f', 'concat', '-safe', '1', '-i', str(out / 'captions.ffconcat'), '-loop', '1', '-i', str(out / 'disclosure.png')]
    if voice:
        command += ['-i', str(out / 'narration.wav')]
    speed = Image.new('RGBA', (1440, 900))
    draw = ImageDraw.Draw(speed)
    draw.rounded_rectangle((1250, 12, 1428, 48), radius=8, fill=(10, 10, 10, 220))
    draw.text((1262, 18), f'{args.gap_rate:g}× sped up', font=small, fill='white')
    speed.save(out / 'speed.png')
    command += ['-loop', '1', '-i', str(out / 'speed.png')]
    crop = 'crop=' + ':'.join(map(str, (args.crop[2], args.crop[3], args.crop[0], args.crop[1]))) + ':exact=1,' if args.crop else ''
    retime = ''
    if any(s['rate'] != 1 for s in segments):
        expression = balanced_sum([f"clip(PTS*TB-{s['source_start']:.12f},0,{s['source_end']-s['source_start']:.12f})/{s['rate']:.12f}" for s in segments])
        retime = f"setpts='{expression}/TB',"
    filters = f"[0:v]setpts=PTS-STARTPTS,{retime}{crop}scale=1440:900:flags=lanczos:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=1440:900:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30[b];[b][1:v]overlay=0:0:eof_action=pass[c];[c][2:v]overlay=0:0[d]"
    enable = balanced_sum([f"gte(t,{s['output_start']:.12f})*lt(t,{s['output_end']:.12f})" for s in segments if s['rate'] > 1])
    filters += f";[d][{4 if voice else 3}:v]overlay=0:0:enable='{enable}'[v]"
    audio_map = '3:a' if voice else None
    if args.source_audio == 'keep' and any(s['codec_type'] == 'audio' for s in info['streams']):
            audio = next(s for s in info['streams'] if s['codec_type'] == 'audio')
            offset = float(audio.get('start_time', 0)) - float(video.get('start_time', 0))
            # amix ignores input start offsets: encode the common video origin
            # as actual silence (or trim pre-video audio), not independent PTS resets.
            align = f'adelay={offset*1000:.6f}:all=1' if offset >= 0 else f'atrim=start={-offset:.9f},asetpts=PTS-STARTPTS'
            filters += f';[0:a]asetpts=PTS-STARTPTS,{align},apad[a0]'
            if voice:
                filters += ';[a0][3:a]amix=inputs=2:duration=longest:normalize=0[a]'
            else:
                filters += ';[a0]anull[a]'
            audio_map = '[a]'
    command += ['-filter_complex', filters, '-map', '[v]']
    command += ['-an'] if audio_map is None else ['-map', audio_map, '-c:a', 'aac', '-b:a', '160k']
    partial = out / 'share.partial.mp4'
    command += ['-t', str(output_duration), '-c:v', 'libx264', '-crf', '20', '-preset', 'fast', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', '-map_metadata', '-1', '-map_chapters', '-1', '-metadata', 'comment=' + label, str(partial)]
    run(*command)
    report = qa(partial, output_duration, audio_map is not None)
    report['output_sha256'] = digest(partial)
    if digest(raw_source) != source_hash or evidence_hashes(args.bundle) != original_hashes:
        raise ValueError('source changed during export; share copy not published')
    (out / 'qa.json').write_text(json.dumps(report, indent=2) + '\n')
    # Exclusive publication, so even a competing writer cannot be overwritten.
    os.link(partial, out / 'share.mp4')
    partial.unlink()
    print(out / 'share.mp4')


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        sys.exit(f"proof-video: {exc}")

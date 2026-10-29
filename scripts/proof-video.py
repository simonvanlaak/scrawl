#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow>=10,<12", "edge-tts>=7,<8"]
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
        raise ValueError('export changed real-time duration')
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
    parser.add_argument('--allow-online-tts', action='store_true', help='consent to send cue text ONLY to Microsoft Edge TTS')
    parser.add_argument('--voice', default='en-GB-RyanNeural')
    parser.add_argument('--rate', default='+10%')
    parser.add_argument('--crop', type=rect, help='source x,y,width,height; static measured close-up, no retiming')
    parser.add_argument('--caption-box', type=rect, default=(40, 730, 1360, 140), help='safe output x,y,width,height; keep results outside this region')
    parser.add_argument('--font', type=Path, required=True, help='regular sans-serif TTF, e.g. DejaVuSans.ttf')
    args = parser.parse_args()
    if args.silent and args.source_audio != 'drop':
        raise ValueError('--silent requires --source-audio drop')
    if not args.silent and not args.allow_online_tts:
        raise ValueError('TTS sends cue text externally: pass --allow-online-tts or --silent')
    from PIL import Image, ImageDraw, ImageFont
    source = args.source.resolve(strict=True)
    source_hash = digest(source)
    info = probe(source)
    video = next(s for s in info['streams'] if s['codec_type'] == 'video')
    duration = float(video.get('duration', info['format']['duration']))
    cues = validate(json.loads(args.cues.read_text()), duration, args.crop, int(video['width']), int(video['height']))
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
        if tw + 30 > w or th + 30 > h:
            raise ValueError('caption exceeds safe box: shorten or insert one line break')
        left, top = x + (w - tw) // 2, y + h - 15 - th
        draw.rounded_rectangle((left-15, top-15, left+tw+15, top+th+15), radius=8, fill=(10, 10, 10, 220))
        draw.multiline_text((left-bb[0], top-bb[1]), text, font=font, fill='white', spacing=7, align='center')
        images.append(image)
    out = args.out_dir.resolve()
    out.mkdir(mode=0o700, parents=False, exist_ok=False)
    (out / 'cues.json').write_text(json.dumps(cues, indent=2) + '\n')
    (out / 'spoken-script.txt').write_text('\n\n'.join(t.replace('\n', ' ') for _, _, t in cues) + '\n')
    (out / 'explanation.srt').write_text('\n'.join(f'{i+1}\n{stamp(a)} --> {stamp(b)}\n{t}\n' for i, (a, b, t) in enumerate(cues)))
    policy = {'source': str(source), 'source_sha256': source_hash, 'source_audio': args.source_audio, 'synthetic_narration': not args.silent, 'voice': None if args.silent else args.voice, 'rate': args.rate, 'crop': args.crop, 'caption_box': args.caption_box, 'duration': duration}
    (out / 'provenance.json').write_text(json.dumps(policy, indent=2) + '\n')
    # All work files stay in the newly created private output directory, even on failure.
    timeline = ['ffconcat version 1.0']
    Image.new('RGBA', (1440, 900)).save(out / 'blank.png')
    previous = 0
    for i, ((a, b, _), image) in enumerate(zip(cues, images)):
        if a > previous:
            timeline += ["file 'blank.png'", f'duration {a-previous:.9f}']
        image.save(out / f'caption-{i:03}.png')
        timeline += [f"file 'caption-{i:03}.png'", f'duration {b-a:.9f}']
        previous = b
    if duration > previous:
        timeline += ["file 'blank.png'", f'duration {duration-previous:.9f}']
    timeline += ["file 'blank.png'"]
    (out / 'captions.ffconcat').write_text('\n'.join(timeline) + '\n')
    timing = []
    if not args.silent:
        import edge_tts
        async def synthesize():
            for i, (_, _, text) in enumerate(cues):
                await edge_tts.Communicate(text.replace('\n', ' '), voice=args.voice, rate=args.rate).save(str(out / f'cue-{i:03}.mp3'))
        asyncio.run(synthesize())
        sample_rate = 48000
        mix = array.array('h', [0]) * round(duration * sample_rate)
        for i, (a, b, text) in enumerate(cues):
            raw = run('ffmpeg', '-v', 'error', '-i', out / f'cue-{i:03}.mp3', '-ar', sample_rate, '-ac', 1, '-f', 's16le', '-')
            samples = array.array('h')
            samples.frombytes(raw)
            seconds = len(samples) / sample_rate
            fit_speech(seconds, a, b)
            start = round((a + 0.05) * sample_rate)
            mix[start:start+len(samples)] = samples
            timing.append({'cue': i, 'start': a+0.05, 'end': a+0.05+seconds, 'text': text, 'tempo': 1})
        if sys.byteorder != 'little':
            mix.byteswap()
        with wave.open(str(out / 'narration.wav'), 'wb') as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(sample_rate)
            wav.writeframes(mix.tobytes())
    (out / 'timing-report.json').write_text(json.dumps(timing, indent=2) + '\n')
    # Visible disclosure survives detachment from this script and its sidecars.
    badge = Image.new('RGBA', (1440, 900))
    draw = ImageDraw.Draw(badge)
    label = 'Explanatory captions' if args.silent else 'Synthetic voice-over · source audio ' + args.source_audio
    small = ImageFont.truetype(str(args.font), 20)
    draw.rounded_rectangle((12, 12, 620, 48), radius=8, fill=(10, 10, 10, 220))
    draw.text((24, 18), label, font=small, fill='white')
    badge.save(out / 'disclosure.png')
    command = ['ffmpeg', '-v', 'error', '-nostdin', '-i', str(source), '-f', 'concat', '-safe', '1', '-i', str(out / 'captions.ffconcat'), '-loop', '1', '-i', str(out / 'disclosure.png')]
    if not args.silent:
        command += ['-i', str(out / 'narration.wav')]
    crop = 'crop=' + ':'.join(map(str, (args.crop[2], args.crop[3], args.crop[0], args.crop[1]))) + ':exact=1,' if args.crop else ''
    filters = f'[0:v]setpts=PTS-STARTPTS,{crop}scale=1440:900:flags=lanczos:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=1440:900:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30[b];[b][1:v]overlay=0:0:eof_action=pass[c];[c][2:v]overlay=0:0[v]'
    audio_map = '3:a'
    if not args.silent:
        if args.source_audio == 'keep' and any(s['codec_type'] == 'audio' for s in info['streams']):
            audio = next(s for s in info['streams'] if s['codec_type'] == 'audio')
            offset = float(audio.get('start_time', 0)) - float(video.get('start_time', 0))
            # amix ignores input start offsets: encode the common video origin
            # as actual silence (or trim pre-video audio), not independent PTS resets.
            align = f'adelay={offset*1000:.6f}:all=1' if offset >= 0 else f'atrim=start={-offset:.9f},asetpts=PTS-STARTPTS'
            filters += f';[0:a]asetpts=PTS-STARTPTS,{align}[a0];[a0][3:a]amix=inputs=2:duration=longest:normalize=0[a]'
            audio_map = '[a]'
    command += ['-filter_complex', filters, '-map', '[v]']
    command += ['-an'] if args.silent else ['-map', audio_map, '-c:a', 'aac', '-b:a', '160k']
    partial = out / 'share.partial.mp4'
    command += ['-t', str(duration), '-c:v', 'libx264', '-crf', '20', '-preset', 'fast', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', '-map_metadata', '-1', '-map_chapters', '-1', '-metadata', 'comment=' + label, str(partial)]
    run(*command)
    report = qa(partial, duration, not args.silent)
    if digest(source) != source_hash:
        raise ValueError('source changed during export; share copy not published')
    (out / 'qa.json').write_text(json.dumps(report, indent=2) + '\n')
    partial.rename(out / 'share.mp4')
    print(out / 'share.mp4')


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        sys.exit(f"proof-video: {exc}")

"""Run with python3 -m unittest discover -s scripts/tests -p test_proof_video.py."""
import importlib.util
from pathlib import Path
import unittest
import json
import os
import subprocess
import sys
import tempfile
import array
import math

# Resolve before subprocesses switch to synthetic fixture directories.
FONT = str(Path(__file__).resolve().parents[2] / 'assets/fonts/HostGrotesk-Variable.ttf')

spec = importlib.util.spec_from_file_location('proof', Path(__file__).parents[1] / 'proof-video.py')
assert spec and spec.loader
proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(proof)


class Validation(unittest.TestCase):
    def test_piecewise_boundaries_adjacent_and_identity(self):
        spans = [(2, 3), (3, 4), (7, 8)]
        segments = proof.timing_map(10, spans, 4, 0.25)
        self.assertEqual(segments[0]['source_start'], 0)
        self.assertEqual(segments[-1]['source_end'], 10)
        for left, right in zip(segments, segments[1:]):
            self.assertEqual(left['source_end'], right['source_start'])
            self.assertEqual(left['output_end'], right['output_start'])
        for s in segments:
            self.assertLess(s['output_start'], s['output_end'])
            self.assertAlmostEqual(proof.mapped(s['source_start'], segments), s['output_start'])
            self.assertAlmostEqual(proof.mapped(s['source_end'], segments), s['output_end'])
        self.assertAlmostEqual(proof.mapped(4, segments)-proof.mapped(2, segments), 2)
        for t in (0, 1, 2, 3, 7, 10):
            self.assertEqual(proof.mapped(t, proof.timing_map(10, spans, 1)), t)
        self.assertEqual(proof.mapped(10, proof.timing_map(10, [], 4)), 2.5)
        for rate, padding, ranges in [(float('nan'), 0, []), (float('inf'), 0, []), (0, 0, []), (17, 0, []), (4, -1, []), (4, float('nan'), []), (4, 0, [(3, 2)]), (4, 0, [(0, 11)]), (4, 0, [(0, float('inf'))])]:
            with self.subTest(rate=rate, padding=padding, ranges=ranges), self.assertRaises(ValueError):
                proof.timing_map(10, ranges, rate, padding)

    def test_speech_budget(self):
        proof.fit_speech(0.8, 0, 1)
        for seconds in (0.91, 1.2):
            with self.assertRaisesRegex(ValueError, 'shorten text'):
                proof.fit_speech(seconds, 0, 1)

    def test_valid_cues_and_crop(self):
        self.assertEqual(proof.validate([[0, 1, 'This is real.'], [2, 3, 'A gap.']], 3, (0, 0, 640, 400), 640, 480), [[0, 1, 'This is real.'], [2, 3, 'A gap.']])

    def test_bad_cues(self):
        for cues in ([], [[0, 2, 'a'], [1, 3, 'b']], [[-1, 1, 'a']], [[0, 4, 'a']], [[0, 0, 'a']], [[0, float('nan'), 'a']], [[True, 1, 'a']], [[0, 1, '']], [[0, 1, 3]], [[0, 1]], [[0, 1, 'a\nb\nc']]):
            with self.subTest(cues=cues), self.assertRaises(ValueError):
                proof.validate(cues, 3, None, 640, 480)

    def test_bad_crops(self):
        for crop in ((-1, 0, 2, 2), (0, 0, 0, 2), (0, 0, 642, 480), (639, 0, 2, 2)):
            with self.subTest(crop=crop), self.assertRaises(ValueError):
                proof.validate([[0, 1, 'A result.']], 3, crop, 640, 480)


class Export(unittest.TestCase):
    def test_many_cues_and_protected_ranges_render_without_expression_limit(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source, cues = root / 'source.mp4', root / 'cues.json'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=160x100:r=30:d=30', '-c:v', 'libx264', source)
            windows = [[0.6*i+0.1, 0.6*i+0.3, 'State.'] for i in range(50)]
            cues.write_text(json.dumps(windows))
            for name, flags in [('default', []), ('compressed', ['--compress-gaps']) , ('protected', ['--compress-gaps', '--no-captions', *[v for a,b,_ in windows for v in ['--protect-range', f'{a},{b}']]])]:
                out = root / name
                p = subprocess.run([sys.executable, str(proof.__file__), str(source), '--cues', str(cues), '--out-dir', str(out), '--source-audio', 'drop', '--no-voice', '--gap-padding', '0', '--font', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', *flags], capture_output=True)
                self.assertEqual(p.returncode, 0, p.stderr.decode()[-2000:])
                policy = json.loads((out / 'provenance.json').read_text())
                self.assertEqual(policy['output_duration'], 30) if not flags else self.assertLess(policy['output_duration'], 30)

    def test_compressed_silent_export_maps_protected_states(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'source.mp4'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=s=640x400:r=30:d=6', '-c:v', 'libx264', source)
            before = proof.digest(source)
            cues = root / 'cues.json'
            cues.write_text('[[1,2,"Visible state."]]')
            out = root / 'out'
            result = subprocess.run([sys.executable, str(proof.__file__), str(source), '--cues', str(cues), '--out-dir', str(out), '--source-audio', 'drop', '--silent', '--no-captions', '--compress-gaps', '--protect-range', '3,4', '--font', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'], capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            policy = json.loads((out / 'provenance.json').read_text())
            self.assertLess(policy['output_duration'], 4)
            proof.qa(out / 'share.mp4', policy['output_duration'], False)
            self.assertEqual(proof.digest(source), before)
            segments = policy['time_map']
            self.assertEqual(segments[0]['source_start'], 0)
            self.assertEqual(segments[-1]['source_end'], 6)
            self.assertTrue(any(s['rate'] == 1 and s['source_start'] <= 3 and s['source_end'] >= 4 for s in segments))
            # Same source state after mapping, below the provenance/speed indicators.
            def pixels(path, t):
                return proof.run('ffmpeg', '-v', 'error', '-ss', t, '-i', path, '-frames:v', 1, '-vf', 'scale=1440:900,crop=100:100:650:400', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-')
            a, b = pixels(source, 3.5), pixels(out / 'share.mp4', proof.mapped(3.5, segments))
            self.assertLess(sum(abs(x-y) for x, y in zip(a, b))/len(a), 12)

    def test_measured_speech_readable_captions_and_independent_optouts(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'source.mp4'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=8', '-c:v', 'libx264', source)
            cues = root / 'cues.json'
            cues.write_text('[[1,4,"Real state."],[5,7,"Next."]]')
            speech = root / 'reviewed'
            speech.mkdir()
            (speech / 'cues.json').write_bytes(cues.read_bytes())
            for i in range(2):
                proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'sine=frequency=880:duration=0.4', speech / f'cue-{i:03}.mp3')
            base = [sys.executable, str(proof.__file__), str(source), '--cues', str(cues), '--source-audio', 'drop', '--font', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf']
            for name, switches in [('compressed', ['--compress-gaps', '--speech-dir', str(speech)]), ('voice-only', ['--compress-gaps', '--speech-dir', str(speech), '--no-captions']), ('caption-only', ['--compress-gaps', '--no-voice']), ('original', ['--no-compression', '--speech-dir', str(speech)])]:
                out = root / name
                p = subprocess.run([*base, '--out-dir', str(out), *switches], capture_output=True)
                self.assertEqual(p.returncode, 0, p.stderr.decode())
                policy = json.loads((out / 'provenance.json').read_text())
                proof.qa(out / 'share.mp4', policy['output_duration'], name != 'caption-only')
                if name == 'original':
                    self.assertEqual(policy['output_duration'], 8)
                else:
                    self.assertLess(policy['output_duration'], 8)
                if name == 'voice-only':
                    self.assertEqual((out / 'explanation.srt').read_text(), '')
                if name in ('compressed', 'voice-only'):
                    times = json.loads((out / 'timing-report.json').read_text())
                    import wave
                    with wave.open(str(out / 'narration.wav')) as wav:
                        pcm = array.array('h')
                        pcm.frombytes(wav.readframes(wav.getnframes()))
                    for t in times:
                        original = array.array('h')
                        original.frombytes(proof.run('ffmpeg', '-v', 'error', '-i', speech / f"cue-{t['cue']:03}.mp3", '-ar', 48000, '-ac', 1, '-f', 's16le', '-'))
                        start = round(t['start']*48000)
                        self.assertEqual(pcm[start:start+len(original)], original, 'narration samples must not stretch or truncate')
                        self.assertAlmostEqual(t['end']-t['start'], t['source_end']-t['source_start'])
                    if name == 'compressed':
                        for a, b, text in policy['output_cues']:
                            self.assertGreaterEqual(b-a, max(1, len(text)/15))
                            self.assertIn(f'{proof.stamp(a)} --> {proof.stamp(b)}', (out / 'explanation.srt').read_text())
                        def pixel(t, crop):
                            return proof.run('ffmpeg', '-v', 'error', '-ss', t, '-i', out / 'share.mp4', '-frames:v', 1, '-vf', crop, '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-')
                        at = proof.mapped(1.2, policy['time_map'])
                        self.assertLess(sum(pixel(0.05, 'crop=2:2:1350:43')), sum(pixel(at, 'crop=2:2:1350:43'))/2, 'speed badge only in accelerated time')
                        self.assertLess(sum(pixel(at, 'crop=2:2:720:862')), sum(pixel(0.05, 'crop=2:2:720:862'))/2, 'burned caption follows mapped cue')
                        audible = array.array('h')
                        audible.frombytes(proof.run('ffmpeg', '-v', 'error', '-ss', times[0]['start']+0.1, '-i', out / 'share.mp4', '-t', 0.1, '-ar', 48000, '-ac', 1, '-f', 's16le', '-'))
                        self.assertGreater(max(abs(v) for v in audible), 500, 'encoded audio at mapped cue')
            for flag, value in [('--gap-rate', 'nan'), ('--gap-rate', 'inf'), ('--gap-padding', '-1'), ('--protect-range', '0,9'), ('--protect-range', 'nan,2'), ('--source-audio', 'keep')]:
                p = subprocess.run([*base, '--out-dir', str(root / 'bad'), '--compress-gaps', '--no-voice', flag, value], capture_output=True)
                self.assertNotEqual(p.returncode, 0)
                self.assertFalse((root / 'bad/share.mp4').exists())
            original_hash = proof.digest(source)
            (speech / 'cues.json').write_text('[[1,4,"Different text."]]')
            p = subprocess.run([*base, '--out-dir', str(root / 'mismatch'), '--speech-dir', str(speech)], capture_output=True)
            self.assertNotEqual(p.returncode, 0)
            self.assertIn('reused speech cues differ', p.stderr.decode())
            self.assertFalse((root / 'mismatch/share.mp4').exists())
            for window, message in [([1,1.2,'Hi.'], 'no time stretching'), ([1,1.6,'This caption needs much more time.'], 'caption window too short')]:
                cues.write_text(json.dumps([window]))
                (speech / 'cues.json').write_bytes(cues.read_bytes())
                out = root / ('bad-speech' if message == 'no time stretching' else 'bad-caption')
                p = subprocess.run([*base, '--out-dir', str(out), '--compress-gaps', '--speech-dir', str(speech)], capture_output=True)
                self.assertNotEqual(p.returncode, 0)
                self.assertIn(message, p.stderr.decode())
                self.assertFalse((out / 'share.mp4').exists())
                self.assertEqual(proof.digest(source), original_hash)

    def test_independent_caption_voice_switches(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'source.mp4'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=1', '-f', 'lavfi', '-i', 'sine=duration=1', '-c:v', 'libx264', '-c:a', 'aac', source)
            cues = root / 'cues.json'
            cues.write_text('[[0,1,"Real evidence."]]')
            for captions in (True, False):
                out = root / str(captions)
                cmd = [sys.executable, proof.__file__, str(source), '--cues', str(cues), '--out-dir', str(out), '--source-audio', 'keep', '--no-voice', '--font', FONT]
                if not captions:
                    cmd += ['--no-captions']
                result = subprocess.run(cmd, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                proof.qa(out / 'share.mp4', 1, True)
                self.assertEqual(json.loads((out / 'provenance.json').read_text())['captions'], captions)

    def test_negative_audio_offset_trims_pre_movie_samples(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'negative.mp4'
            proof.run('ffmpeg', '-v', 'error', '-itsoffset', 1, '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=3', '-f', 'lavfi', '-i', 'sine=duration=2', '-c:v', 'libx264', '-c:a', 'aac', '-copyts', source)
            info = proof.probe(source)
            video = next(s for s in info['streams'] if s['codec_type'] == 'video')
            audio = next(s for s in info['streams'] if s['codec_type'] == 'audio')
            self.assertLess(float(audio['start_time']) - float(video['start_time']), -0.9)
            cues = root / 'cues.json'
            cues.write_text('[[0,1,"The real screen."]]')
            subprocess.run([sys.executable, proof.__file__, str(source), '--cues', str(cues), '--out-dir', str(root / 'out'), '--source-audio', 'keep', '--no-voice', '--font', FONT], check=True, capture_output=True)
            def rms(t):
                samples = array.array('h')
                samples.frombytes(proof.run('ffmpeg', '-v', 'error', '-ss', t, '-i', root / 'out/share.mp4', '-t', 0.15, '-ar', 48000, '-ac', 1, '-f', 's16le', '-'))
                return math.sqrt(sum(s*s for s in samples) / len(samples))
            self.assertGreater(rms(0.3), 500)
            self.assertLess(rms(1.5), 20)

    def test_narration_keep_drop_preserve_delayed_audio_and_strip_metadata(self):
        with tempfile.TemporaryDirectory(prefix='proof-audio-', dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'raw.mp4'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=3', '-itsoffset', 1, '-f', 'lavfi', '-i', 'sine=frequency=440:duration=0.5', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-metadata', 'title=Private source title', source)
            before = source.read_bytes()
            # Replace only the online provider with deterministic fixture speech.
            (root / 'edge_tts.py').write_text('''import subprocess
class Communicate:
    def __init__(self, text, voice, rate):
        pass
    async def save(self, path):
        subprocess.run(['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'sine=frequency=880:duration=0.2', path], check=True)
''')
            cues = root / 'cues.json'
            cues.write_text('[[2,3,"Fixture speech."]]')
            for policy, captions in (('keep', True), ('drop', True), ('drop', False)):
                out = root / (policy + str(captions))
                switches = [] if captions else ['--no-captions']
                subprocess.run([sys.executable, str(proof.__file__), str(source), '--cues', str(cues), '--out-dir', str(out), '--source-audio', policy, '--allow-online-tts', '--font', FONT, *switches], env={**os.environ, 'PYTHONPATH': str(root)}, check=True, capture_output=True)
                self.assertEqual(json.loads((out / 'provenance.json').read_text())['captions'], captions)
                movie = out / 'share.mp4'
                def rms(t):
                    samples = array.array('h')
                    samples.frombytes(proof.run('ffmpeg', '-v', 'error', '-ss', t, '-i', movie, '-t', 0.15, '-ar', 48000, '-ac', 1, '-f', 's16le', '-'))
                    return math.sqrt(sum(s*s for s in samples) / len(samples))
                self.assertLess(rms(0.3), 20, 'delayed source must not move to video start')
                if policy == 'keep':
                    self.assertGreater(rms(1.15), 500, 'source onset stays at its recorded time')
                else:
                    self.assertLess(rms(1.15), 20, 'drop must remove source audio')
                self.assertGreater(rms(2.1), 500, 'fixture narration stays at its cue')
                self.assertNotIn('title', proof.probe(movie)['format'].get('tags', {}))
                self.assertEqual(source.read_bytes(), before)

    def test_real_silent_export_and_refusals_preserve_source(self):
        # Run under uv --with Pillow for this real ffmpeg integration check.
        with tempfile.TemporaryDirectory(prefix='proof-test-', dir=os.environ.get('TMPDIR')) as folder:
            root = Path(folder)
            source = root / 'raw.mp4'
            proof.run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=3', '-f', 'lavfi', '-i', 'sine=frequency=440:duration=3', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-c:a', 'aac', source)
            before = source.read_bytes()
            cues = root / 'cues.json'
            cues.write_text(json.dumps([[0, 0.8, 'Start.'], [2, 3, 'Result.']]))
            command = [sys.executable, str(Path(proof.__file__)), str(source), '--cues', str(cues), '--out-dir', str(root / 'out'), '--silent', '--source-audio', 'drop', '--font', FONT]
            subprocess.run(command, check=True, capture_output=True)
            exported = root / 'out/share.mp4'
            proof.qa(exported, 3, False)
            self.assertEqual(source.read_bytes(), before)
            self.assertEqual((root / 'out').stat().st_mode & 0o777, 0o700)
            self.assertTrue((root / 'out/explanation.srt').exists())
            # Retain the existing sub-millisecond cue-end tolerance at the movie boundary.
            cues.write_text(json.dumps([[0, 0.8, 'Start.'], [2, 3.0005, 'Result.']]))
            tolerated = command.copy()
            tolerated[tolerated.index('--out-dir') + 1] = str(root / 'tolerated')
            result = subprocess.run(tolerated, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            # A gap must really remove the subtitle, not repeat the last caption.
            def pixel(t):
                return proof.run('ffmpeg', '-v', 'error', '-ss', t, '-i', exported, '-frames:v', 1, '-vf', 'crop=2:2:720:850', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-')
            self.assertNotEqual(pixel(0.4), pixel(1.5))
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 1)
            self.assertEqual(source.read_bytes(), before)
            cues.write_text('[[0,4,"Too late."]]')
            command[command.index('--out-dir') + 1] = str(root / 'bad')
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
            self.assertFalse((root / 'bad').exists())
            self.assertEqual(source.read_bytes(), before)
            with self.assertRaisesRegex(ValueError, 'duration'):
                proof.qa(exported, 20, False)
            # Faststart's moov must precede media data.
            data = exported.read_bytes()
            self.assertLess(data.index(b'moov'), data.index(b'mdat'))


if __name__ == '__main__':
    unittest.main()

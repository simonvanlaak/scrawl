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
            for policy in ('keep', 'drop'):
                out = root / policy
                subprocess.run([sys.executable, str(proof.__file__), str(source), '--cues', str(cues), '--out-dir', str(out), '--source-audio', policy, '--allow-online-tts', '--font', FONT], env={**os.environ, 'PYTHONPATH': str(root)}, check=True, capture_output=True)
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

"""Real installed-CLI integration. Set APPARENT_TEST_BINARY to a staged candidate.
No TTS network calls: provider fixtures test assembly; live TTS is separate proof.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from typing import Any

BINARY = os.environ.get('APPARENT_TEST_BINARY')


@unittest.skipUnless(BINARY, 'set APPARENT_TEST_BINARY to the exact built candidate')
class WalkthroughCli(unittest.TestCase):
    def test_installed_dispatch_clock_preservation_and_export(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            bundle = root / 'take.apparent'
            bundle.mkdir()
            def cli(*args, ok=True) -> Any:
                assert BINARY is not None
                p = subprocess.run([BINARY, 'walkthrough', *map(str, args)], capture_output=True, cwd=root)
                if ok:
                    self.assertEqual(p.returncode, 0, p.stderr.decode())
                    return json.loads(p.stdout)
                self.assertNotEqual(p.returncode, 0)
                return p.stderr.decode()
            self.assertEqual(cli('inspect', bundle)['status'], 'finalizing_or_incomplete')
            subprocess.run(['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=blue:s=640x400:r=30:d=3', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', str(bundle / 'recording.mp4')], check=True)
            manifest = {'version': 1, 'id': 'fixture', 'createdAtMs': 0, 'durationMs': 3400,
                'target': {'mode': 'display', 'displayId': 0, 'frame': {'x': 0, 'y': 0, 'width': 640, 'height': 400, 'scale': 1}},
                'transcript': {'source': 'test', 'text': '', 'segments': []}, 'strokes': [], 'marks': [], 'events': [],
                'keyframes': [{'index': 1, 't': 1400, 'reason': 'initial', 'file': 'frames/frame-01.png', 'event': 'initial view'}],
                'contactSheet': 'contact-sheet.png', 'video': 'recording.mp4', 'videoOffsetMs': 400}
            (bundle / 'apparent.json').write_text(json.dumps(manifest))
            template = root / 'cues.json'
            state = cli('inspect', bundle, '--template', template)
            self.assertEqual(state['status'], 'ready')
            self.assertEqual(state['frames'][0]['movieSeconds'], 1)
            self.assertEqual(json.loads(template.read_text()), [[0, 3, '']])
            cli('inspect', bundle, '--template', template, ok=False)
            export = ['export', bundle, '--experiment', 'agentWalkthrough', '--cues', template, '--out-dir', root / 'out', '--source-audio', 'drop', '--silent']
            self.assertIn('cue', cli(*export, ok=False))
            self.assertFalse((root / 'out/share.mp4').exists())
            template.write_text('[[0,1,"The real screen."],[2,3,"The take ends."]]')
            # Embedded Python must not execute modules from the task's directory.
            (root / 'PIL.py').write_text('raise RuntimeError("untrusted local module executed")')
            before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in bundle.iterdir()}
            result = cli(*export)
            self.assertTrue(Path(result['output']).is_file())
            self.assertEqual(before, {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in bundle.iterdir()})
            provenance = json.loads(Path(result['provenance']).read_text())
            self.assertEqual(provenance['evidence_sha256'], before)
            cli(*export, ok=False)
            forbidden = export.copy()
            forbidden[forbidden.index('--out-dir') + 1] = bundle / 'share'
            self.assertIn('outside', cli(*forbidden, ok=False))
            self.assertFalse((bundle / 'share').exists())
            no_consent = export[:-1]
            no_consent[no_consent.index('--out-dir') + 1] = root / 'no-consent'
            self.assertIn('--allow-online-tts', cli(*no_consent, ok=False))
            self.assertFalse((root / 'no-consent').exists())

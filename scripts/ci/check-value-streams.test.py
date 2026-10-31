"""Contract checks for the thin CI selector (stdlib only, no builds)."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/ci/check-value-streams.sh"


class Entrypoints(unittest.TestCase):
    def plan(self, stream):
        result = subprocess.run(
            ["bash", str(SCRIPT), "--plan", stream], cwd=ROOT,
            text=True, capture_output=True, check=True,
        )
        self.assertTrue(result.stdout.strip())
        return result.stdout

    def test_unknown_or_missing_stream_fails_before_execution(self):
        for args in [[], ["unknown"], ["--plan", "unknown"],
                     ["capture-engine", "extra"], ["--plan"]]:
            with self.subTest(args=args):
                result = subprocess.run(["bash", str(SCRIPT), *args], cwd=ROOT,
                                        text=True, capture_output=True)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("usage:", result.stderr)

    def test_engine_never_builds_the_root_gui(self):
        plan = self.plan("capture-engine")
        commands = plan.splitlines()
        self.assertEqual(len(commands), 3)
        for command in commands:
            self.assertIn("--manifest-path capture/engine/Cargo.toml", command)
        self.assertIn("cargo build", plan)
        self.assertIn("cargo test", plan)
        self.assertIn("--locked -j 3 -- --test-threads=2", plan)
        self.assertNotIn("--workspace", plan)

    def test_connect_is_standalone_and_uses_the_complete_package_test(self):
        plan = self.plan("connect-web")
        self.assertIn("(connect/web) env", plan)
        self.assertIn("npm test", plan)
        self.assertNotIn("(site)", plan)
        self.assertNotIn("(hosted)", plan)
        self.assertIn("test -s dist/connect/index.html", plan)

    def test_hosted_uses_synthetic_suite_and_fresh_worker_output(self):
        plan = self.plan("hosted-service")
        self.assertIn("(hosted) npm test", plan)
        self.assertLess(plan.index("rm -f dist/worker.js"), plan.index("npm test"))
        self.assertIn("test -s dist/worker.js", plan)
        self.assertNotIn("wrangler", plan)
        self.assertTrue(list((ROOT / "hosted/test").glob("*.test.mjs")))

    def test_public_composition_and_nonempty_existing_tests(self):
        plan = self.plan("public-site")
        self.assertIn("APPARENT_PUBLIC_SITE=1", plan)
        self.assertIn("VITE_HOSTED_API_ORIGIN=https://hosted.tryapparent.com", plan)
        self.assertIn("node scripts/build-public.mjs", plan)
        self.assertIn("npm run test:artifact", plan)
        tests = next(line for line in plan.splitlines() if "node --test " in line)
        selected = tests.split("node --test ", 1)[1].split()
        self.assertGreater(len(selected), 0)
        self.assertIn("scripts/test-composed-package.mjs", selected)
        self.assertIn("scripts/test-product-boundaries.mjs", selected)
        for path in selected:
            self.assertTrue((ROOT / "site" / path).is_file(), path)
        self.assertIn("test -s dist-apparent/index.html", plan)

    def test_execution_rejects_failed_build_and_stale_or_empty_output(self):
        # Disposable stub tests selector control flow, not Worker correctness.
        for status in [23, 0]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                (root / "scripts/ci").mkdir(parents=True)
                script = root / "scripts/ci/check-value-streams.sh"
                shutil.copyfile(SCRIPT, script)
                (root / "hosted/dist").mkdir(parents=True)
                worker = root / "hosted/dist/worker.js"
                worker.write_text("stale output")
                (root / "bin").mkdir()
                npm = root / "bin/npm"
                npm.write_text(f"#!/bin/sh\nexit {status}\n")
                npm.chmod(0o700)
                result = subprocess.run(
                    ["bash", str(script), "hosted-service"], text=True,
                    capture_output=True,
                    env={**os.environ, "PATH": f"{root / 'bin'}:{os.environ['PATH']}"},
                )
                self.assertEqual(result.returncode, status or 1)
                self.assertFalse(worker.exists(), "stale output must not satisfy readback")


if __name__ == "__main__":
    unittest.main()

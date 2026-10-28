"""Contract checks for the isolated, manual Mac preview release path."""

from pathlib import Path
import os
import plistlib
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class PreviewReleaseTest(unittest.TestCase):
    def test_packager_layout_and_version_with_stubbed_mac_tools(self):
        # A Linux layout test: the real signing, DMG and GUI still need a Mac run.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            (root / "bin").mkdir()
            (root / "scripts/build-mac-preview.sh").write_bytes(
                (ROOT / "scripts/build-mac-preview.sh").read_bytes()
            )
            (root / "bin/cargo").write_text(
                '#!/bin/sh\nmkdir -p target/release\n'
                'printf "test" > target/release/scrawl\n'
                'printf "test" > target/release/scrawl-helper\n'
            )
            (root / "bin/plutil").write_text('#!/bin/sh\nexit 0\n')
            (root / "bin/codesign").write_text('#!/bin/sh\nexit 0\n')
            (root / "bin/hdiutil").write_text(
                '#!/bin/sh\n'
                'if [ "$1" = create ]; then eval "out=\\${$#}"; printf "fake dmg" > "$out"; fi\n'
            )
            for tool in (root / "bin").iterdir():
                tool.chmod(0o755)
            env = dict(os.environ, PATH=f'{root / "bin"}:{os.environ["PATH"]}',
                       SCRAWL_PREVIEW_VERSION="0.123.2", SCRAWL_PREVIEW_BUILD_NUMBER="123.2")
            result = subprocess.run(["bash", str(root / "scripts/build-mac-preview.sh")],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            info = plistlib.loads((root / "dist/Scrawl Preview.app/Contents/Info.plist").read_bytes())
            self.assertEqual(info["CFBundleIdentifier"], "dev.simonvanlaak.scrawl.preview")
            self.assertEqual(info["CFBundleShortVersionString"], "0.123.2")
            self.assertEqual(info["CFBundleVersion"], "123.2")
            self.assertNotIn("SUFeedURL", info)
            self.assertTrue((root / "dist/Scrawl-Preview-0.123.2-arm64.dmg.sha256").is_file())
            self.assertTrue((root / "dist/preview-staging/Applications").is_symlink())

    def test_helper_embedded_identity_follows_preview_identity(self):
        script = (ROOT / "scripts/build-mac-preview.sh").read_text()
        build = (ROOT / "build.rs").read_text()
        self.assertIn("SCRAWL_HELPER_BUNDLE_ID=dev.simonvanlaak.scrawl.preview.helper", script)
        self.assertIn('rerun-if-env-changed=SCRAWL_HELPER_BUNDLE_ID', build)
        self.assertIn('env::var("SCRAWL_HELPER_BUNDLE_ID")', build)

    def test_preview_cannot_replace_production_cli_or_login_item(self):
        script = (ROOT / "scripts/build-mac-preview.sh").read_text()
        cli = (ROOT / "src/command_line.rs").read_text()
        settings = (ROOT / "src/settings.rs").read_text()
        self.assertIn("SCRAWL_PREVIEW_BUILD=1", script)
        self.assertIn('option_env!("SCRAWL_PREVIEW_BUILD")', cli)
        self.assertIn('option_env!("SCRAWL_PREVIEW_BUILD")', settings)

    def test_preview_bundle_is_separate_and_has_no_production_updater(self):
        script = (ROOT / "scripts/build-mac-preview.sh").read_text()
        self.assertIn("dev.simonvanlaak.scrawl.preview", script)
        self.assertIn("Scrawl Preview.app", script)
        self.assertNotIn("dev.kitlangton.scrawl", script)
        self.assertNotIn("SUFeedURL", script)
        self.assertNotIn("Sparkle.framework", script)
        self.assertNotIn("notarytool", script)
        self.assertNotIn("site/public", script)
        self.assertIn("codesign --verify", script)
        self.assertIn("hdiutil verify", script)

    def test_manual_release_uses_unique_version_and_publishes_only_after_build(self):
        workflow = (ROOT / ".github/workflows/release-mac-preview.yml").read_text()
        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotIn("  push:", workflow)
        self.assertIn("github.ref == 'refs/heads/main'", workflow)
        self.assertIn("macos-26", workflow)
        self.assertIn("github.run_number", workflow)
        self.assertIn("github.run_attempt", workflow)
        self.assertIn("--prerelease", workflow)
        self.assertIn("--latest=false", workflow)
        self.assertIn("gh release create", workflow)
        self.assertLess(workflow.index("build-mac-preview.sh"), workflow.index("gh release create"))
        self.assertRegex(workflow, r'--target "\$GITHUB_SHA"')
        self.assertNotIn("--clobber", workflow)
        self.assertNotIn("release-app.sh", workflow)


if __name__ == "__main__":
    unittest.main()

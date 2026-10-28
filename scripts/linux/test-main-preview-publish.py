#!/usr/bin/env python3
"""Exercise the release workflow's publication gate without publishing anything."""
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/release-linux.yml"
SHA = "a" * 40
HEAD = "b" * 40


def publish_step():
    lines = WORKFLOW.read_text().splitlines()
    start = lines.index("      - name: Publish both checked AppImages")
    marker = lines.index("        run: |", start) + 1
    end = next((i for i in range(marker, len(lines)) if lines[i] and not lines[i].startswith("          ")), len(lines))
    return textwrap.dedent("\n".join(lines[marker:end]))


def run_case(head, comparison, expected, published=""):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "dist").mkdir()
        for arch in ("x86_64", "aarch64"):
            (root / "dist" / f"Apparent-0.3.13-linux-{arch}.AppImage").touch()
        mock = root / "gh"
        mock.write_text("""#!/bin/sh
case "$*" in
  *'/git/ref/heads/main'*) printf '%s\\n' "$MOCK_HEAD" ;;
  *'/releases?per_page=100'*) if [ -n "$MOCK_PUBLISHED" ]; then printf 'linux-main-%s\\n' "$MOCK_PUBLISHED"; fi ;;
  *'/compare/'*) printf '%s\\n' "$MOCK_COMPARISON" ;;
  'release create '*) printf 'PUBLISHED_RELEASE\\n' ;;
  *) exit 91 ;;
esac
""")
        mock.chmod(0o700)
        result = subprocess.run(["bash", "-c", publish_step()], cwd=root, capture_output=True,
                                text=True, env={**os.environ, "PATH": f"{root}:{os.environ['PATH']}",
                                                "MOCK_HEAD": head, "MOCK_COMPARISON": comparison,
                                                "MOCK_PUBLISHED": published,
                                                "GITHUB_SHA": SHA, "GITHUB_REPOSITORY": "tryapparent/apparent"})
        assert (result.returncode == 0 and "PUBLISHED_RELEASE" in result.stdout) == expected, (head, comparison, result)


run_case(SHA, "identical", True)
run_case(HEAD, "ahead", True)  # A checked main ancestor must not starve when main advances.
run_case(HEAD, "diverged", False)
run_case(HEAD, "ahead", False, HEAD)  # Never publish an older preview after a newer one.
assert "ref: ${{ inputs.tag || github.sha }}" in WORKFLOW.read_text(), "build checkout must be pinned to release identity"
print("main-preview publication gate: same head, ancestor, divergence, newer preview passed")

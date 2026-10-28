#!/usr/bin/env python3
"""Run a private, disposable Omarchy GUI test from this checkout."""

import argparse
from pathlib import Path
import platform
import subprocess
import sys
import uuid


def main():
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenario", choices=("update",),
                        help="update clicks the packaged Settings button in Omarchy")
    parser.add_argument("--binary", type=Path, default=None,
                        help="packaged candidate (default: target/appimage/Apparent-<arch>.AppImage)")
    parser.add_argument("--results", type=Path, help="private new result directory")
    parser.add_argument("--job-timeout", type=int, default=900, help="hard lifetime in seconds (default: 900)")
    parser.add_argument("--memory", type=int, default=3072, help="guest MiB (default: 3072)")
    args = parser.parse_args()
    if args.binary is None:
        args.binary = root / f"target/appimage/Apparent-{platform.machine()}.AppImage"
    if args.binary.name != f"Apparent-{platform.machine()}.AppImage":
        parser.error("update requires a packaged Apparent-<arch>.AppImage (--binary)")
    template = Path.home() / "VMs/scrawl-gui-template"
    results = args.results or (Path.home() / "VMs/scrawl-gui-runs" / ("gui-" + uuid.uuid4().hex))
    scenario = root / "scripts/linux/gui-update-settings.sh"
    # Keep explicit inputs so the runner's own existence and template checks fail closed.
    command = [sys.executable, str(root / "scripts/linux/gui-vm.py"),
               "--template", str(template / "system.qcow2"),
               "--ovmf-vars", str(template / "OVMF_VARS.fd"),
               "--ssh-user", "scrawl", "--ssh-key", str(template / "test_access"),
               "--binary", str(args.binary), "--scenario", str(scenario),
               "--results", str(results), "--memory", str(args.memory),
               "--job-timeout", str(args.job_timeout)]
    command.extend(("--qmp-input", "--isolated-network"))
    print("results:", results, flush=True)
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Make a separate immutable GUI template from a powered-off Omarchy guest."""

import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys

spec = importlib.util.spec_from_file_location("gui_vm", Path(__file__).with_name("gui-vm.py"))
assert spec is not None and spec.loader is not None
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path, help="powered-off guest qcow2")
    parser.add_argument("--ovmf-vars", required=True, type=Path)
    parser.add_argument("--destination", required=True, type=Path, help="new template directory")
    args = parser.parse_args()
    if not args.source.is_file() or not args.ovmf_vars.is_file():
        parser.error("source disk and OVMF vars must exist")
    if args.destination.exists():
        parser.error("destination already exists")
    if list(runner.qemu_users(args.source)) or list(runner.qemu_users(args.ovmf_vars)):
        parser.error("source is active in QEMU; arrange a shutdown before preparing a template")
    args.destination.mkdir(parents=True, mode=0o700)
    args.destination.chmod(0o700)
    try:
        disk = args.destination / "system.qcow2"
        subprocess.run(["qemu-img", "convert", "-O", "qcow2", str(args.source), str(disk)], check=True)
        if list(runner.qemu_users(args.source)) or list(runner.qemu_users(args.ovmf_vars)):
            raise RuntimeError("source started during copy; discarding inconsistent template")
        vars_copy = args.destination / "OVMF_VARS.fd"
        shutil.copy2(args.ovmf_vars, vars_copy)
        disk.chmod(0o400)
        vars_copy.chmod(0o400)
        print(args.destination.resolve())
        return 0
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        for filename in ("system.qcow2", "OVMF_VARS.fd"):
            (args.destination / filename).unlink(missing_ok=True)
        args.destination.rmdir()
        print(f"template preparation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

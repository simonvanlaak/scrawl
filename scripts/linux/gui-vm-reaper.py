#!/usr/bin/env python3
"""Expire a disposable GUI guest even if its runner was killed."""

import os
from pathlib import Path
import signal
import sys
import time


def qemu_users(disk):
    """Find only QEMU processes whose drive argument names this exact job disk."""
    target = ("file=" + str(disk.resolve())).encode()
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            args = (proc / "cmdline").read_bytes().split(b"\0")
        except OSError:
            continue
        if not args or b"qemu-system-" not in args[0]:
            continue
        if any(target in arg.split(b",") for arg in args[1:]):
            yield int(proc.name)


def reap(directory):
    directory = directory.resolve()
    disk = directory / "disk.qcow2"
    if not disk.exists():
        return
    owners = list(qemu_users(disk))
    for pid in owners:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    for _ in range(10):
        if not list(qemu_users(disk)):
            break
        time.sleep(0.5)
    else:
        for pid in list(qemu_users(disk)):
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        for _ in range(10):
            if not list(qemu_users(disk)):
                break
            time.sleep(0.5)
    if list(qemu_users(disk)):
        raise RuntimeError("QEMU still owns job disk; refusing to unlink it")
    for name in ("disk.qcow2", "OVMF_VARS.fd", "known_hosts", "qmp.sock", "vnc.sock"):
        (directory / name).unlink(missing_ok=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: gui-vm-reaper.py JOB_DIRECTORY")
    reap(Path(sys.argv[1]))

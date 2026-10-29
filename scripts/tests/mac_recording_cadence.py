#!/usr/bin/env python3
"""Check a real timed Mac recording, not just its container's declared duration.

Run on a bundle produced by `apparent record --window Finder --duration 6`.
The packet timeline must reach the end of capture; editing only the MP4's
declared duration cannot satisfy this check. ScreenCaptureKit can omit idle
frames, so a maximum inter-packet gap is not a valid failure condition.
"""

import argparse
import json
from pathlib import Path
import subprocess
import sys


def probe(movie: Path) -> dict:
    result = subprocess.run(
        [
            "ffprobe", "-v", "error", "-select_streams", "v:0",
            "-show_entries", "format=duration:packet=pts_time,dts_time,duration_time",
            "-show_packets", "-of", "json", str(movie),
        ],
        capture_output=True, text=True, check=True,
    )
    return json.loads(result.stdout)


def check(bundle: Path, expected_seconds: float) -> None:
    manifest = json.loads((bundle / "apparent.json").read_text())
    movie = bundle / "recording.mp4"
    data = probe(movie)
    packets = data["packets"]
    assert len(packets) >= 2, f"only {len(packets)} video packets"
    dts = [float(packet["dts_time"]) for packet in packets]
    assert all(a < b for a, b in zip(dts, dts[1:])), "packet DTS regressed"
    gaps = [b - a for a, b in zip(dts, dts[1:])]
    max_gap = max(gaps)
    # B-frames put early DTS below zero. The distance from the first to the
    # last packet is the useful live sample span, independent of container
    # metadata or a final packet whose duration was extended after capture.
    span = dts[-1] - dts[0]
    assert span >= expected_seconds - 0.5, (
        f"last real packet at {span:.3f}s of {expected_seconds:.3f}s requested"
    )
    duration = float(data["format"]["duration"])
    assert duration >= expected_seconds - 0.5, (
        f"MP4 {duration:.3f}s for {expected_seconds:.3f}s requested"
    )
    assert duration <= expected_seconds + 1.5, f"MP4 unexpectedly long: {duration:.3f}s"
    actual = manifest["durationMs"] / 1000
    assert actual >= expected_seconds - 0.1, f"capture stopped early: {actual:.3f}s"
    print(f"{bundle}: {len(packets)} packets, max DTS gap {max_gap:.3f}s, "
          f"MP4 {duration:.3f}s, bundle {actual:.3f}s")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--seconds", type=float, default=6)
    args = parser.parse_args()
    try:
        check(args.bundle, args.seconds)
    except (AssertionError, OSError, KeyError, ValueError, subprocess.CalledProcessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

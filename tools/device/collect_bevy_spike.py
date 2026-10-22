#!/usr/bin/env python3
"""Collect and verify the latest completed launcher Bevy spike run over SSH."""
from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    output = (args.output or ROOT / ".scratch/bevy-spike/runs" /
              datetime.now().strftime("%Y%m%d-%H%M%S-launcher")).resolve()
    output.mkdir(parents=True, exist_ok=False)
    remote = r"""set -eu
cd /mnt/SDCARD/Apps/boat-bevy-wgpu
run=$(cat save/last-run.txt)
printf '%s\n' "$run" | grep -Eq '^run-[0-9]{8}-[0-9]{6}-[0-9]+$' || exit 2
if [ ! -f "save/$run/exit-code.txt" ]; then
    printf 'Latest run has not finished: %s\n' "$run" >&2
    exit 3
fi
sha256sum boat >&2
cat /sys/kernel/debug/pvr/status >&2
tar -czf - build.json save/last-run.txt "save/$run"
"""
    with (output / "results.tar.gz").open("wb") as archive:
        result = subprocess.run([str(ROOT / "ssh.sh"), "-o", "PubkeyAuthentication=no",
                                 "-o", "IdentityAgent=none", remote], stdout=archive,
                                stderr=subprocess.PIPE, timeout=120)
    (output / "device-status.txt").write_bytes(result.stderr)
    if result.returncode:
        print(result.stderr.decode(errors="replace"))
        raise SystemExit(f"Collection failed ({result.returncode}); evidence: {output}")
    with tarfile.open(output / "results.tar.gz") as archive:
        archive.extractall(output, filter="data")
    run = (output / "save/last-run.txt").read_text().strip()
    if not re.fullmatch(r"run-\d{8}-\d{6}-\d+", run):
        raise SystemExit("Invalid run identifier")
    run_dir = output / "save" / run
    build = json.loads((run_dir / "build.json").read_text())
    installed = re.search(rb"^([a-f0-9]{64})  boat$", result.stderr, re.MULTILINE)
    if not installed or installed[1].decode() != build["binary_sha256"]:
        raise SystemExit("Installed binary does not match collected build metadata")
    code = int((run_dir / "exit-code.txt").read_text())
    log = (run_dir / "spike.log").read_text()
    manifest = {"collected_at": datetime.now().astimezone().isoformat(), "run": run,
                "exit_code": code, "binary_sha256": build["binary_sha256"],
                "files_sha256": {str(p.relative_to(output)): hashlib.sha256(p.read_bytes()).hexdigest()
                                 for p in sorted(output.rglob("*")) if p.is_file()}}
    (output / "collection.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Results: {run_dir}")
    print(log)
    if code or "WATCHDOG:" in log or "DISPLAY_FAILED" in log:
        raise SystemExit(f"Launcher run did not finish cleanly (exit {code})")
    if "stage=display_snapshot_saved" not in log:
        raise SystemExit("No final surface readback; an early gamepad exit is not a pixel-verified run")
    images = sorted(run_dir.glob("display*.ppm"))
    for image in images:
        subprocess.run([sys.executable, str(ROOT / "tools/device/verify_bevy_spike_ppm.py"),
                        str(image)], check=True)
    final_pose = json.loads((run_dir / "display.json").read_text()) if (run_dir / "display.json").is_file() else {}
    if final_pose.get("animated"):
        poses = [json.loads(p.with_suffix(".json").read_text()) for p in images]
        poses.sort(key=lambda p: p["presented_frame"])
        positions = [p["sprite_center_x"] for p in poses]
        frames = [p["scene_frame"] for p in poses]
        if len(poses) < 4 or max(positions) - min(positions) < 128 or any(b <= a for a, b in zip(frames, frames[1:])):
            raise SystemExit(f"Insufficient animation evidence: frames={frames} positions={positions}")
        print(f"BEVY_ANIMATION_PASSED frames={frames} sprite_center_x={positions}")


if __name__ == "__main__":
    main()

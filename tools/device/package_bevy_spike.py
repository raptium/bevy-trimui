#!/usr/bin/env python3
"""Package the Bevy wgpu display spike as a launcher App for the Brick.

The device's launcher always owns the panel. A present test run directly over
SSH shares the display with the launcher and flickers, so it cannot be judged
by eye. Apps started from the launcher get the screen, so the visual check
ships in the same layout as the game:

  dist/Apps/boat-bevy-wgpu/
    boat         the cross-built spike binary
    launch.sh    runs --display for a bounded time, then exits to the launcher
    config.json  launcher metadata (scanned from <sdcard>/Apps/*/config.json)
    icon.png     the same icon generator the game uses
    save/        HOME for the app

The game has a separate package at dist/Apps/boat-native.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
APP = "boat-bevy-wgpu"
BINARY = ROOT / ".scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/boat-bevy-spike"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--seconds", type=int, default=120,
                    help="how long launch.sh runs the display test before exiting")
    ap.add_argument("--binary", type=Path, default=BINARY,
                    help="cross-built spike to package, including an isolated upgrade candidate")
    ap.add_argument("--dest", type=Path, default=ROOT / "dist/Apps",
                    help="directory that receives the app folder")
    args = ap.parse_args()
    if not 1 <= args.seconds <= 3600:
        ap.error("--seconds must be 1..3600")

    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit("Build experiments/bevy-spike first: python3 tools/device/build_bevy_spike.py")
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    build_file = binary.with_suffix(".build.json")
    if not build_file.is_file():
        raise SystemExit("Missing build metadata; rebuild with build_bevy_spike.py or build_bevy_020.py")
    build_info = json.loads(build_file.read_text())
    if build_info.get("binary_sha256") != binary_hash:
        raise SystemExit("Binary differs from build metadata; rebuild the probe")

    package = args.dest / APP
    if package.exists():
        shutil.rmtree(package)
    (package / "save").mkdir(parents=True)

    shutil.copy2(binary, package / "boat")
    (package / "boat").chmod(0o755)
    shutil.copy2(build_file, package / "build.json")

    launch = package / "launch.sh"
    launch.write_text(f"""#!/bin/sh
progdir=/mnt/SDCARD/Apps/{APP}
export LD_LIBRARY_PATH="/rom/usr/trimui/lib:$LD_LIBRARY_PATH"
export HOME="$progdir/save"
# The launcher owns the panel until an app starts; run the wgpu display test
# and always give the panel back. Exit: Guide or Start on the pad, the time
# limit below, or the process-level watchdog in the binary. The last presented
# frame, log and exit code are kept per run so a previous image cannot be
# mistaken for the current run. No launcher or system service is stopped.
run_name="run-$(date +%Y%m%d-%H%M%S)-$$"
run_dir="$progdir/save/$run_name"
mkdir -p "$run_dir" || exit 1
printf '%s\\n' "$run_name" > "$progdir/save/last-run.txt"
cd "$run_dir" || exit 1
cp "$progdir/build.json" build.json || exit 1
printf 'stage=launcher_run seconds={args.seconds}\\n' > spike.log
"$progdir/boat" --display --animate --seconds {args.seconds} >> spike.log 2>&1
code=$?
printf '%s\\n' "$code" > exit-code.txt
exit "$code"
""")
    launch.chmod(0o755)

    (package / "config.json").write_text(json.dumps({
        "package": "com.boat.bevy.wgpu",
        "label": "boat (Bevy WGPU)",
        "label.ch.lang": "小船 (Bevy WGPU 测试)",
        "icontop": "icon.png",
        "icon": "",
        "launch": "launch.sh",
        "description": "Native Bevy wgpu renderer on VK_KHR_display (display spike)",
    }, indent=4, ensure_ascii=False) + "\n")

    subprocess.run([sys.executable, str(ROOT / "tools/make_icon.py"), str(package / "icon.png")], check=True)
    print(f"Packaged {package}")
    subprocess.run(["du", "-sh", str(package)], check=True)


if __name__ == "__main__":
    main()

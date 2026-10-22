#!/usr/bin/env python3
"""Cross-build the isolated wgpu experiment, with explicit patch provenance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("--stock", action="store_true", help="Build without PowerVR workaround")
ap.add_argument("--display", action="store_true", help="Also enable raw display surface import")
args = ap.parse_args()
if args.stock and args.display:
    ap.error("--stock and --display are mutually exclusive")
env = os.environ.copy()
for key, relative in (("CARGO_TARGET_DIR", "target"), ("ZIG_LOCAL_CACHE_DIR", "zig-local"), ("ZIG_GLOBAL_CACHE_DIR", "zig-global")):
    env[key] = str(ROOT / ".scratch/wgpu-probe" / relative)
# C dependencies must use the target compiler even on a fresh host.
env["PATH"] = f"{ROOT / 'cross'}:{env['PATH']}"
env["CC_aarch64_unknown_linux_gnu"] = str(ROOT / "cross/aarch64-linux-gnu-gcc")
env["AR_aarch64_unknown_linux_gnu"] = "zig ar"
crates = []
if not args.stock:
    prepare = "prepare_wgpu_display.py" if args.display else "prepare_wgpu_hal.py"
    subprocess.run([sys.executable, str(ROOT / "tools/device" / prepare)], check=True)
    crates = ["wgpu-hal", "wgpu-core", "wgpu"] if args.display else ["wgpu-hal"]
cmd = ["cargo", "build", "--manifest-path", "experiments/wgpu-probe/Cargo.toml", "--target", "aarch64-unknown-linux-gnu", "--release", "--offline"]
for crate in crates:
    cmd += ["--config", f'patch.crates-io.{crate}.path=".scratch/wgpu-probe/vendor/{crate}"']
if args.display:
    cmd += ["--features", "display-import"]
subprocess.run(cmd, cwd=ROOT, env=env, check=True)
binary = ROOT / ".scratch/wgpu-probe/target/aarch64-unknown-linux-gnu/release/boat-wgpu-probe"
sources = [ROOT / "experiments/wgpu-probe" / p for p in ("Cargo.toml", "Cargo.lock")]
sources += sorted((ROOT / "experiments/wgpu-probe/src").rglob("*.*"))
sources += [ROOT / path for path in (
    ".cargo/config.toml", "cross/zig-cc-wrapper", "cross/aarch64-linux-gnu-gcc",
    "tools/device/build_wgpu.py", "tools/device/prepare_wgpu_hal.py",
    "tools/device/prepare_wgpu_display.py",
)]
metadata = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "build_command": cmd,
            "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "zig": subprocess.check_output(["zig", "version"], text=True).strip(),
            "patched_crates": crates, "display_import": args.display,
            "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
            "patch_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in (ROOT / ".scratch/wgpu-probe/vendor").glob("*.patch")
                             if any(p.name.startswith(crate + "-") for crate in crates)
                             or (p.name == "pvr-timeline-values.patch" and "wgpu-hal" in crates)}}
binary.with_suffix(".build.json").write_text(json.dumps(metadata, indent=2) + "\n")
print(f"Built {binary}")

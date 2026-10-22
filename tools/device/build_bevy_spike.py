#!/usr/bin/env python3
"""Cross-build the native Bevy probe with real dependencies and local patches.

The demo uses crates/bevy-trimui for display and readiness. Driver
compatibility changes remain in isolated wgpu vendor copies; no registry edit.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("--offline", action="store_true")
args = ap.parse_args()
env = os.environ.copy()
for key, relative in (("CARGO_TARGET_DIR", "target"), ("ZIG_LOCAL_CACHE_DIR", "zig-local"),
                      ("ZIG_GLOBAL_CACHE_DIR", "zig-global")):
    env[key] = str(ROOT / ".scratch/bevy-spike" / relative)
# C dependencies and the Rust linker use the uv-locked Zig via cross/zig.
env["PATH"] = f"{ROOT / 'cross'}:{env['PATH']}"
env["CC_aarch64_unknown_linux_gnu"] = str(ROOT / "cross/aarch64-linux-gnu-gcc")
env["AR_aarch64_unknown_linux_gnu"] = "zig ar"

fetch = ["cargo", "fetch", "--manifest-path", "experiments/bevy-spike/Cargo.toml", "--target", "aarch64-unknown-linux-gnu"]
if args.offline:
    fetch += ["--offline"]
subprocess.run(fetch, cwd=ROOT, env=env, check=True)
cargo_home = env.get("CARGO_HOME", str(Path.home() / ".cargo"))
configs = []
subprocess.run([sys.executable, str(ROOT / "tools/device/prepare_wgpu_display.py"),
                "--cargo-home", cargo_home], check=True)
crates = ["wgpu-hal", "wgpu-core", "wgpu"]
for crate in crates:
    configs.append(f'patch.crates-io.{crate}.path=".scratch/wgpu-probe/vendor/{crate}"')

cmd = ["cargo", "build", "--manifest-path", "experiments/bevy-spike/Cargo.toml",
       "--target", "aarch64-unknown-linux-gnu", "--release"]
if args.offline:
    cmd += ["--offline"]
for config in configs:
    cmd += ["--config", config]
subprocess.run(cmd, cwd=ROOT, env=env, check=True)
binary = ROOT / ".scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/bevy-trimui-demo"
sources = [ROOT / "experiments/bevy-spike" / p for p in ("Cargo.toml", "Cargo.lock")]
sources += sorted((ROOT / "experiments/bevy-spike/src").rglob("*.*"))
sources += [ROOT / "Cargo.toml", ROOT / "crates/bevy-trimui/Cargo.toml"]
sources += sorted((ROOT / "crates/bevy-trimui/src").rglob("*.*"))
sources += [ROOT / path for path in (
    ".cargo/config.toml", "cross/zig", "cross/zig-cc-wrapper", "cross/aarch64-linux-gnu-gcc",
    "pyproject.toml", "uv.lock",
    "tools/device/build_bevy_spike.py", "tools/device/prepare_wgpu_hal.py",
    "tools/device/prepare_wgpu_display.py",
)]
metadata = {
    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "build_command": cmd,
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "zig": subprocess.check_output(["zig", "version"], env=env, text=True).strip(),
    "patched_crates": crates,
    "dependency_resolution": "real crate archives; no optional stubs",
    "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
    "patch_sha256": {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in (ROOT / ".scratch/wgpu-probe/vendor").glob("*.patch")
        if p.name == "pvr-timeline-values.patch" or p.name.endswith("display-import.patch")
    },
}
binary.with_suffix(".build.json").write_text(json.dumps(metadata, indent=2) + "\n")
print(f"Built {binary}")

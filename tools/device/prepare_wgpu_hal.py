#!/usr/bin/env python3
"""Prepare an isolated wgpu-hal copy with the PowerVR submit workaround."""
from pathlib import Path
import hashlib
import json
import shutil
import argparse
import os

ROOT = Path(__file__).resolve().parents[2]
ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("--version", default="29.0.4")
ap.add_argument("--cargo-home", type=Path, default=Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))))
ap.add_argument("--vendor-dir", type=Path, default=ROOT / ".scratch/wgpu-probe/vendor")
args = ap.parse_args()
matches = sorted((args.cargo_home / "registry/src").glob(f"*/wgpu-hal-{args.version}"))
if len(matches) != 1:
    raise SystemExit(f"Expected exactly one cached wgpu-hal {args.version}; cargo fetch first")
source = matches[0]
dest = args.vendor_dir.resolve() / "wgpu-hal"
dest.mkdir(parents=True, exist_ok=True)
shutil.copytree(source, dest, dirs_exist_ok=True)
path = dest / "src/vulkan/semaphore_list.rs"
original = path.read_bytes()
old = b"        let timeline_info = timeline_info.write(vk::TimelineSemaphoreSubmitInfo::default());"
new = b"""        // PowerVR DDK 1.19@6345021 dereferences the value arrays even for
        // binary semaphores whenever timeline submit info is present.
        // Give those semaphores dummy values; Vulkan ignores them.
        if !wait_semaphores.values.is_empty() || !signal_semaphores.values.is_empty() {
            wait_semaphores.pad_values();
            signal_semaphores.pad_values();
        }

        let timeline_info = timeline_info.write(vk::TimelineSemaphoreSubmitInfo::default());"""
if original.count(old) != 1:
    raise SystemExit("Upstream source did not match expected patch site")
patched = original.replace(old, new)
path.write_bytes(patched)
import difflib
patch = "".join(difflib.unified_diff(original.decode().splitlines(True), patched.decode().splitlines(True),
                                   fromfile="a/src/vulkan/semaphore_list.rs", tofile="b/src/vulkan/semaphore_list.rs"))
(dest.parent / "pvr-timeline-values.patch").write_text(patch)
(dest.parent / "provenance.json").write_text(json.dumps({
    "upstream": f"wgpu-hal {args.version}", "source": str(source),
    "original_sha256": hashlib.sha256(original).hexdigest(),
    "patched_sha256": hashlib.sha256(patched).hexdigest(),
}, indent=2) + "\n")
print(f"Prepared {dest}")

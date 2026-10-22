#!/usr/bin/env python3
"""Cross-build and run the no-window Vulkan capability probe on the device."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--snapshot", type=Path, required=True)
    ap.add_argument("--sdk", type=Path, default=Path(".sdk"),
                    help="SDK root containing usr/include/vulkan (default: .sdk)")
    args = ap.parse_args()
    snapshot = args.snapshot.resolve()
    if not snapshot.is_dir():
        raise SystemExit("Snapshot directory does not exist")
    device_headers = snapshot / "rootfs/usr/include/vulkan"
    headers = device_headers if device_headers.is_dir() else args.sdk / "usr/include/vulkan"
    if not headers.is_dir():
        raise SystemExit("Vulkan headers missing; pass --sdk with a header-equipped SDK")
    build = ROOT / ".scratch/device-probe"
    include = build / "include"
    include.mkdir(parents=True, exist_ok=True)
    shutil.copytree(headers, include / "vulkan", dirs_exist_ok=True)
    env = os.environ.copy()
    env["ZIG_LOCAL_CACHE_DIR"] = str(build / "zig-local")
    env["ZIG_GLOBAL_CACHE_DIR"] = str(build / "zig-global")
    binary = build / "vulkan-probe"
    source = ROOT / "tools/device/vulkan_probe.c"
    cmd = ["zig", "cc", "-target", "aarch64-linux-gnu.2.25", "-I", str(include),
           "-O2", str(source), "-ldl", "-o", str(binary)]
    subprocess.run(cmd, env=env, check=True)
    remote = """set -eu
probe_path=$(mktemp /tmp/boat-vulkan-probe.XXXXXX)
trap 'rm -f "$probe_path"' EXIT
cat > "$probe_path"
chmod 700 "$probe_path"
LD_LIBRARY_PATH=/rom/usr/trimui/lib:/usr/lib:/lib "$probe_path"
"""
    with binary.open("rb") as inp, (snapshot / "vulkan-probe.txt").open("wb") as out, \
            (snapshot / "vulkan-probe-stderr.txt").open("wb") as err:
        result = subprocess.run([str(ROOT / "ssh.sh"), remote], stdin=inp, stdout=out,
                                stderr=err, timeout=45)
    with source.open("rb") as src, binary.open("rb") as exe:
        provenance = {"source": str(source.relative_to(ROOT)),
                      "source_sha256": hashlib.file_digest(src, "sha256").hexdigest(),
                      "binary_sha256": hashlib.file_digest(exe, "sha256").hexdigest(),
                      "header_source": str(headers), "build_command": cmd,
                      "exit_code": result.returncode,
                      "scope": "instance and physical-device enumeration only; no logical device, surface, draw or present"}
    (snapshot / "vulkan-probe-build.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print((snapshot / "vulkan-probe.txt").read_text())
    if result.returncode:
        raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Inspect a snapshot offline: ELF dependencies and differences from the SDK."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import struct


def elf_metadata(path: Path) -> dict | None:
    with path.open("rb") as stream:
        header = stream.read(64)
        if header[:4] != b"\x7fELF":
            return None
        if header[4:6] != b"\x02\x01":
            return {"note": "not ELF64 little-endian"}
        data = header + stream.read()
    h = struct.unpack_from("<HHIQQQIHHHHHH", data, 16)
    result = {"machine": h[1], "needed": [], "soname": None}
    if not h[5] or not h[11]:
        return result
    sections = [struct.unpack_from("<IIQQQQIIQQ", data, h[5] + i * h[10]) for i in range(h[11])]
    for sec in sections:
        if sec[1] != 6:  # SHT_DYNAMIC
            continue
        strings = sections[sec[6]]
        def string_at(offset: int) -> str:
            start = strings[4] + offset
            return data[start:data.index(b"\0", start)].decode("utf-8", "replace")
        for offset in range(sec[4], sec[4] + sec[5], sec[9] or 16):
            tag, value = struct.unpack_from("<qQ", data, offset)
            if tag == 1:
                result["needed"].append(string_at(value))
            elif tag == 14:
                result["soname"] = string_at(value)
            elif tag in (15, 29):
                result["rpath" if tag == 15 else "runpath"] = string_at(value)
    return result


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("snapshot", type=Path)
    ap.add_argument("--sdk", type=Path, default=Path(".sdk"),
                    help="optional SDK root for comparison (default: .sdk)")
    args = ap.parse_args()
    sysroot = args.snapshot / "rootfs"
    report = {}
    for path in sorted(sysroot.rglob("*")):
        if path.is_file() and not path.is_symlink():
            meta = elf_metadata(path)
            if meta is not None:
                report[str(path.relative_to(sysroot))] = meta
    (args.snapshot / "elf-metadata.json").write_text(json.dumps(report, indent=2) + "\n")
    comparisons = []
    for name in ("libvulkan.so", "libVK_IMG.so", "libEGL.so", "libGLESv2.so", "libIMGegl.so", "libPVRScopeServices.so"):
        device = sysroot / "usr/lib" / name
        sdk = args.sdk / "usr/lib" / name
        comparisons.append({"name": name, "device_exists": device.is_file(), "sdk_exists": sdk.is_file(),
                            "device_sha256": digest(device) if device.is_file() else None,
                            "sdk_sha256": digest(sdk) if sdk.is_file() else None})
    device_sdl = sysroot / "rom/usr/trimui/lib/libSDL2-2.0.so.0"
    sdk_sdl = args.sdk / "usr/lib/libSDL2-2.0.so.0"
    comparisons.append({"name": "SDL2", "device_library": str(device_sdl.resolve().relative_to(sysroot.resolve())),
                        "sdk_library": str(sdk_sdl.resolve()),
                        "device_exists": device_sdl.is_file(), "sdk_exists": sdk_sdl.is_file(),
                        "device_sha256": digest(device_sdl) if device_sdl.is_file() else None,
                        "sdk_sha256": digest(sdk_sdl) if sdk_sdl.is_file() else None})
    (args.snapshot / "sdk-comparison.json").write_text(json.dumps(comparisons, indent=2) + "\n")
    versions = {}
    for label, prefix in (("sdk", args.sdk / "usr/lib"), ("device", sysroot / "usr/lib")):
        path = prefix / "libGLESv2.so"
        if path.is_file():
            versions[label] = sorted({s.decode("ascii") for s in re.findall(rb"[ -~]{6,}", path.read_bytes())
                                      if re.search(rb"OpenGL ES.*build [0-9]+\.[0-9]+@[0-9]+", s)})
    (args.snapshot / "gpu-build-strings.json").write_text(json.dumps(versions, indent=2) + "\n")
    print(f"Inspected {len(report)} ELF files; wrote elf-metadata.json and sdk-comparison.json")


if __name__ == "__main__":
    main()

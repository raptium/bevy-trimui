#!/usr/bin/env python3
"""Read-only runtime/header snapshot through the project's SSH wrapper.

The device need not have rsync. Tar streams preserve its symlinks and file modes;
local extraction rebases absolute symlinks to stay inside the snapshot sysroot.
No network/account configuration, user data or full /etc tree is collected.
"""
from __future__ import annotations

import argparse
import datetime as dt
import gzip
import hashlib
import json
import os
from pathlib import Path
import posixpath
import shlex
import subprocess
import tarfile
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[2]
SSH = ROOT / "ssh.sh"

# Only packaged runtime directories and a small explicit metadata allowlist.
# /lib's configuration/network scripts are deliberately outside the selection.
SELECTION = """
for p in /usr/lib /usr/include /rom/usr/trimui/lib /lib/modules /lib/firmware /lib/security /lib/udev /usr/share/vulkan /etc/vulkan /usr/share/glvnd /etc/openwrt_release /etc/openwrt_version /etc/version /etc/ld.so.conf /etc/ld.so.conf.d /etc/ld.so.cache; do
  if [ -e "$p" ]; then printf '%s\\n' "${p#/}"; fi
done
for p in /lib/*.so* /lib/*.a /lib/*.o; do
  if [ -e "$p" ] || [ -L "$p" ]; then printf '%s\\n' "${p#/}"; fi
done
"""

REPORT = """
printf '== kernel ==\\n'; uname -a; cat /proc/version
printf '\\n== firmware ==\\n'
for p in /etc/openwrt_release /etc/openwrt_version /etc/version; do
  if [ -f "$p" ]; then printf '%s\\n' "$p"; cat "$p"; fi
done
printf '\\n== GPU driver ==\\n'
cat /sys/kernel/debug/pvr/version /sys/kernel/debug/pvr/status /sys/kernel/debug/pvr/gpu00/info 2>/dev/null
printf '\\n== loaded modules ==\\n'; cat /proc/modules
printf '\\n== graphics nodes ==\\n'; ls -l /dev/dri /dev/fb* 2>/dev/null
printf '\\n== DRM driver names ==\\n'
for p in /sys/kernel/debug/dri/*/name; do [ ! -f "$p" ] || cat "$p"; done
printf '\\n== graphics package metadata ==\\n'
for p in /usr/lib/opkg/info/libsdl2.control /usr/lib/opkg/info/libc.control /usr/lib/opkg/info/opengles_demo.control; do
  [ ! -f "$p" ] || cat "$p"
done
printf '\\n== runtime sizes ==\\n'; du -sk /lib /usr/lib /usr/include /rom/usr/trimui/lib
printf '\\n== tools ==\\n'; command -v rsync; command -v vulkaninfo; command -v readelf
true
"""


def remote(command: str, dest: Path) -> None:
    with dest.open("wb") as out:
        subprocess.run([str(SSH), command], stdout=out, check=True)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    timestamp = dt.datetime.now(ZoneInfo("Asia/Shanghai")).strftime("%Y%m%d-%H%M%S")
    out = (args.out or ROOT / ".device-snapshots" / f"brick-pro-{timestamp}").resolve()
    out.mkdir(parents=True, exist_ok=False)
    print(f"Snapshot: {out}", flush=True)
    remote(SELECTION, out / "selection.txt")
    selected = (out / "selection.txt").read_text().splitlines()
    if not selected or any(p.startswith("/") or ".." in p.split("/") for p in selected):
        raise RuntimeError("Invalid remote selection")
    remote(REPORT, out / "system-report.txt")
    remote("cat /proc/config.gz", out / "kernel-config.gz")
    (out / "kernel-config.txt").write_bytes(gzip.decompress((out / "kernel-config.gz").read_bytes()))
    # Filesystem names are fixed by the selection or trusted vendor library names.
    paths = " ".join(shlex.quote(p) for p in selected)
    checksum_cmd = f"cd / && find {paths} -type f -exec sha256sum {{}} \\;"
    remote(checksum_cmd, out / "remote-files.sha256")
    print("Streaming runtime libraries, headers and modules...", flush=True)
    archive = out / "rootfs.tar.gz"
    proc = subprocess.Popen([str(SSH), f"cd / && tar -cf - {paths}"], stdout=subprocess.PIPE)
    try:
        with gzip.open(archive, "wb", compresslevel=3) as packed:
            assert proc.stdout is not None
            while chunk := proc.stdout.read(1024 * 1024):
                packed.write(chunk)
        if proc.wait() != 0:
            raise RuntimeError("Remote tar failed")
    finally:
        if proc.stdout:
            proc.stdout.close()
        if proc.poll() is None:
            proc.kill()
            proc.wait()
    sysroot = out / "rootfs"
    sysroot.mkdir()
    with tarfile.open(archive, "r:gz") as tf:
        members = tf.getmembers()
        for member in members:
            if member.issym() and member.linkname.startswith("/"):
                member.linkname = posixpath.relpath(member.linkname.lstrip("/"), posixpath.dirname(member.name) or ".")
        tf.extractall(sysroot, members=members, filter="data")
    verified = 0
    for line in (out / "remote-files.sha256").read_text().splitlines():
        digest, filename = line.split("  ", 1)
        path = sysroot / filename
        with path.open("rb") as stream:
            local_digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if local_digest != digest:
            raise RuntimeError(f"Checksum mismatch: {filename}")
        verified += 1
    links = {str(p.relative_to(sysroot)): os.readlink(p) for p in sysroot.rglob("*") if p.is_symlink()}
    (out / "symlinks.json").write_text(json.dumps(links, indent=2) + "\n")
    with archive.open("rb") as stream:
        archive_digest = hashlib.file_digest(stream, "sha256").hexdigest()
    manifest = {
        "captured_at": dt.datetime.now(ZoneInfo("Asia/Shanghai")).isoformat(),
        "selected_paths": selected,
        "verified_regular_files": verified,
        "symlinks": len(links),
        "archive_bytes": archive.stat().st_size,
        "archive_sha256": archive_digest,
        "note": "Runtime snapshot; device headers may be incomplete. Absolute symlinks rebased only in extracted rootfs. Original symlinks retained in archive.",
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2), flush=True)


if __name__ == "__main__":
    main()

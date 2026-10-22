#!/usr/bin/env python3
"""Install a packaged TRIMUI launcher App onto its SD card over SSH.

Streams a compressed tar into a temporary folder, verifies the binary hash,
then replaces the app while preserving its save directory and run evidence. The
launcher scans <sdcard>/Apps/*/config.json, so the app appears in its list the
next time it refreshes. The device must be running; nothing is changed on it
except that one app folder.
"""
from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path
import shlex
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SSH = ROOT / "ssh.sh"
APP = "bevy-trimui-demo"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--package", type=Path, default=ROOT / "dist/Apps" / APP)
    ap.add_argument("--app-name", default=APP,
                    help="launcher directory name")
    ap.add_argument("--executable", default=APP,
                    help="executable filename inside the package")
    ap.add_argument("--remote-dir", default="/mnt/SDCARD/Apps",
                    help="Apps directory on the device's SD card")
    args = ap.parse_args()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", args.app_name):
        ap.error("--app-name must be a lowercase launcher directory name")
    app = args.app_name
    executable = args.executable
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", executable):
        ap.error("--executable must be a simple filename")

    package = args.package.resolve()
    for required in (executable, "launch.sh", "config.json"):
        if not (package / required).is_file():
            raise SystemExit(f"{package}/{required} missing; run the appropriate package_* tool first")

    remote = args.remote_dir.rstrip("/")
    digest = hashlib.sha256((package / executable).read_bytes()).hexdigest()
    remote_cmd = "set -eu\ncd " + shlex.quote(remote) + "\n" + r"""
stage=$(mktemp -d .APP_NAME-install.XXXXXX)
cleanup() {
    if [ -e "$stage/previous" ] && [ ! -e APP_NAME ]; then
        mv "$stage/previous" APP_NAME
    fi
    rm -rf "$stage"
}
trap cleanup EXIT HUP INT TERM
tar -xzf - -C "$stage"
test -x "$stage/APP_NAME/EXECUTABLE_NAME"
test -f "$stage/APP_NAME/config.json"
printf '%s  %s\n' BINARY_HASH "$stage/APP_NAME/EXECUTABLE_NAME" | sha256sum -c -
if [ -d APP_NAME/save ]; then
    cp -a APP_NAME/save/. "$stage/APP_NAME/save/"
fi
if [ -e APP_NAME ]; then mv APP_NAME "$stage/previous"; fi
if ! mv "$stage/APP_NAME" APP_NAME; then
    if [ -e "$stage/previous" ]; then mv "$stage/previous" APP_NAME; fi
    exit 1
fi
ls -l APP_NAME
"""
    substitutions = {"APP_NAME": app, "EXECUTABLE_NAME": executable, "BINARY_HASH": digest}
    remote_cmd = re.sub(r"\b(APP_NAME|EXECUTABLE_NAME|BINARY_HASH)\b",
                        lambda match: substitutions[match[0]], remote_cmd)
    with tempfile.TemporaryFile() as payload:
        with tarfile.open(fileobj=payload, mode="w:gz", compresslevel=1) as archive:
            archive.add(package, arcname=app)
        print(f"Installing {package} -> {remote}/{app} ({payload.tell()} compressed bytes)", flush=True)
        payload.seek(0)
        result = subprocess.run([str(SSH), "-o", "PubkeyAuthentication=no",
                                 "-o", "IdentityAgent=none", remote_cmd], stdin=payload, timeout=240)
    if result.returncode:
        raise SystemExit("install failed; see ssh output above")
    print(f"Installed launcher App: {app}")
    print("Run evidence is saved in save/run-*/; save/last-run.txt identifies the latest run.")


if __name__ == "__main__":
    main()

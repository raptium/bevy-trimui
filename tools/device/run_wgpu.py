#!/usr/bin/env python3
"""Run the isolated wgpu probe on the handheld and collect logs/readback."""
from __future__ import annotations

import argparse
from datetime import datetime
import gzip
import hashlib
import json
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--binary", type=Path, default=ROOT / ".scratch/wgpu-probe/target/aarch64-unknown-linux-gnu/release/bevy-trimui-wgpu-probe")
    ap.add_argument("--output", type=Path)
    ap.add_argument("--timeout", type=int, default=240,
                    help="SSH timeout including compressed binary transfer and execution")
    ap.add_argument("--env", action="append", default=[], metavar="NAME=VALUE",
                    help="environment for the isolated program, e.g. RUST_LOG=info")
    ap.add_argument("probe_args", nargs="*")
    args = ap.parse_args()
    environment = {}
    for item in args.env:
        name, separator, value = item.partition("=")
        if not separator or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            ap.error("--env requires NAME=VALUE with a valid environment variable name")
        if name in ("HOME", "CODEX_HOME"):
            ap.error(f"--env cannot repurpose {name}")
        environment[name] = value
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit("Build experiments/wgpu-probe first; see its README")
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    build_file = binary.with_suffix(".build.json")
    build_info = json.loads(build_file.read_text()) if build_file.is_file() else None
    if build_info and build_info["binary_sha256"] != binary_hash:
        raise SystemExit("Binary differs from build metadata; rebuild using build_wgpu.py")
    unavailable_sources = []
    if build_info and build_info.get("source_sha256"):
        source_files = []
        for name in build_info["source_sha256"]:
            path = ROOT / name
            if path.is_file():
                source_files.append(path)
            else:
                unavailable_sources.append(name)
    else:
        source_files = list((ROOT / "experiments/wgpu-probe/src").rglob("*.*"))
        source_files += [ROOT / "experiments/wgpu-probe" / p for p in ("Cargo.toml", "Cargo.lock")]
    source_hashes = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in source_files}
    output = (args.output or ROOT / ".scratch/wgpu-probe/runs" / datetime.now().strftime("%Y%m%d-%H%M%S")).resolve()
    output.mkdir(parents=True, exist_ok=False)
    remote = r"""set -u
run_dir=$(mktemp -d /tmp/bevy-trimui-wgpu-probe.XXXXXX) || exit 1
child=
watcher=
cleanup() {
    if [ -n "$child" ]; then kill "$child" 2>/dev/null || true; fi
    if [ -n "$watcher" ]; then kill "$watcher" 2>/dev/null || true; fi
    rm -rf "$run_dir"
}
trap cleanup EXIT HUP INT TERM
gzip -dc > "$run_dir/probe" || exit 1
chmod 700 "$run_dir/probe"
cd "$run_dir" || exit 1
cat /sys/kernel/debug/pvr/status > gpu-before.txt 2>&1 || true
LD_LIBRARY_PATH=/rom/usr/trimui/lib:/usr/lib:/lib RUST_BACKTRACE=1 RUST_LOG=info PROBE_ENVIRONMENT ./probe PROBE_ARGUMENTS > stdout.txt 2> stderr.txt &
child=$!
(sleep 45; kill "$child" 2>/dev/null; sleep 3; kill -9 "$child" 2>/dev/null) </dev/null >/dev/null 2>&1 &
watcher=$!
wait "$child"
code=$?
child=
kill "$watcher" 2>/dev/null || true
wait "$watcher" 2>/dev/null || true
watcher=
printf '%s\n' "$code" > exit-code.txt
cat /sys/kernel/debug/pvr/status > gpu-after.txt 2>&1 || true
rm -f probe
tar -cf - .
""".replace("PROBE_ARGUMENTS", shlex.join(args.probe_args)).replace(
        "PROBE_ENVIRONMENT", " ".join(f"{name}={shlex.quote(value)}" for name, value in environment.items()))
    provenance = {"binary": str(binary), "binary_sha256": binary_hash,
                  "args": args.probe_args, "build": build_info,
                  "source_at_run_sha256": source_hashes,
                  "unavailable_sources": unavailable_sources,
                  "transport": "gzip", "environment": environment}
    provenance_file = output / "provenance.json"
    provenance_file.write_text(json.dumps(provenance, indent=2) + "\n")
    with tempfile.TemporaryFile() as inp:
        with binary.open("rb") as source, gzip.GzipFile(fileobj=inp, mode="wb", compresslevel=1) as compressed:
            shutil.copyfileobj(source, compressed)
        print(f"Uploading {binary.name}: {binary.stat().st_size} -> {inp.tell()} compressed bytes", flush=True)
        inp.seek(0)
        with (output / "results.tar").open("wb") as out:
            try:
                # The wrapper supplies password auth; the desktop key agent
                # can otherwise delay each connection by over a minute.
                result = subprocess.run([str(ROOT / "ssh.sh"), "-o", "PubkeyAuthentication=no",
                                         "-o", "IdentityAgent=none", remote], stdin=inp, stdout=out,
                                        stderr=subprocess.PIPE, timeout=args.timeout)
            except subprocess.TimeoutExpired as error:
                (output / "ssh-stderr.txt").write_bytes(error.stderr or b"")
                provenance["transport_error"] = f"SSH timed out after {args.timeout}s; no render result established"
                provenance_file.write_text(json.dumps(provenance, indent=2) + "\n")
                raise SystemExit(f"SSH timed out; partial evidence: {output}") from None
    (output / "ssh-stderr.txt").write_bytes(result.stderr)
    if result.returncode:
        raise SystemExit(f"SSH failed ({result.returncode}); see {output}")
    with tarfile.open(output / "results.tar") as archive:
        archive.extractall(output, filter="data")
    code = int((output / "exit-code.txt").read_text())
    provenance["exit_code"] = code
    provenance_file.write_text(json.dumps(provenance, indent=2) + "\n")
    print(f"Results: {output}")
    lines = (output / "stdout.txt").read_text().splitlines()
    if len(lines) > 35:
        print("\n".join(lines[:8] + [f"... {len(lines) - 28} lines saved in stdout.txt ..."] + lines[-20:]))
    else:
        print("\n".join(lines))
    print((output / "stderr.txt").read_text())
    if code:
        raise SystemExit(code)


if __name__ == "__main__":
    main()

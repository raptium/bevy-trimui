#!/usr/bin/env python3
"""Refresh the current source inventory while retaining original provenance."""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    path = ROOT / "SOURCE-MANIFEST.json"
    previous = json.loads(path.read_text())
    origin = previous.get("origin", {
        "private_source_revision": previous.get("source_revision"),
        "initial_public_commit": "0801f5e76f950ebdc5d2949feedb3b2894360aee",
    })
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=ROOT
    ).decode().split("\0")
    hashes = {}
    for name in sorted(set(names) - {"", path.name}):
        source = ROOT / name
        if source.is_symlink():
            raise SystemExit(f"Source inventory does not accept symlinks: {name}")
        if source.is_file():
            hashes[name] = hashlib.sha256(source.read_bytes()).hexdigest()
    path.write_text(json.dumps({
        "schema_version": 2,
        "kit": "bevy-trimui-brick-pro",
        "origin": origin,
        "note": "Current source-file inventory, excluding this manifest and ignored generated files. Original revision identifies pre-extraction provenance, not a device test. Refresh with uv run tools/update_source_manifest.py.",
        "files_sha256": hashes,
    }, indent=2) + "\n")
    print(f"Updated source inventory: {len(hashes)} files")


if __name__ == "__main__":
    main()

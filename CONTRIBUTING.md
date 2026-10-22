# Contributing

Keep changes focused on the platform, probes and repeatable workflows. A report
is most useful when it includes the device model, firmware/build, GPU DDK,
Rust/Zig versions, crate lockfiles, exact command, binary hash and final verdict.
For a rendering failure include stdout/stderr, exit code, GPU status before and
after, and the matching readback/pose files. Distinguish transport failures from
program failures and panel observations from pixel checks.

For local validation:

```sh
uv run python -m compileall -q tools
uv run python -m unittest discover -s tests
cargo fmt --manifest-path experiments/bevy-spike/Cargo.toml -- --check
uv run tools/device/build_bevy_spike.py
uv run tools/device/package_bevy_spike.py --seconds 30
uv run tools/update_source_manifest.py
```

Keep Python tooling dependencies in `pyproject.toml` and commit `uv.lock`.
The local transport test uses a temporary filesystem and an SSH fixture; it
exercises packaging, save preservation and real pixel verification without
connecting to hardware. Build metadata hashes the uv lockfile and Zig wrapper
as well as the Rust sources and driver patches.

A build-only contribution can explicitly say that device execution was not
performed. A new compatibility claim needs its own dated device run; the
reference results cannot establish support for another firmware. Submit only
selected, sanitized logs; exclude addresses, credentials, user saves and device
libraries. Keep `docs/compatibility.json` and the relevant task documentation
consistent with new evidence. Preserve upstream notices in patched copies.

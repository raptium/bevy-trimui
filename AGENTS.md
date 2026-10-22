# Agent entrypoint

This is an asset-free Brick Pro platform kit. Run commands from this checkout's
root with `uv run`; dependencies belong in `pyproject.toml` and `uv.lock`.
Use the project environment for Python tools; do not add requirements files or
manual environment setup. `SOURCE-MANIFEST.json` records provenance, not a test run.

Read only what the task needs:

- Build/run: [README.md](README.md), then [example](experiments/bevy-spike/README.md).
- Port an app: [integration](docs/integration.md).
- Research hardware/firmware: [device](docs/device.md) and [compatibility.json](docs/compatibility.json).
- Audio: [audio](docs/audio.md) and `tools/device/brick_audio.py`.
- Diagnose a failure: [troubleshooting](docs/troubleshooting.md).
- Interpret or report tests: [evidence](docs/evidence.md).
- Reusable task instructions: [Skill](skills/trimui-brick-bevy/SKILL.md).

Preserve these constraints:

- The supported baseline is Bevy **0.19.1**, wgpu **29.0.4**. Do not upgrade as
  an incidental fix. Match the HAL/core/wgpu versions when preparing patches.
- Run the Python build entrypoint; plain `cargo build` with `brick-display`
  lacks private surface-import APIs until local patches are configured.
- Patches go into `.scratch/` copies, never the Cargo registry or device `.so`s.
  Patch-anchor mismatches require source review, not a fuzzy replacement.
- The camera uses an image target and `Msaa::Off`. Wait for pipeline readiness;
  a fixed frame count is not a reliable screenshot gate.
- A raw surface must be created with the same Vulkan instance and transferred
  once to wgpu. Do not independently destroy it after transfer.
- Use a launcher App for physical display acceptance. Direct SSH offscreen
  tests are useful; direct presentation contends with the launcher.
- Installation and run tools act on a real device. Use the user's intended
  target and authorized deployment scope. Do not stop launcher/services or
  overwrite system graphics/audio configuration to make a test pass.
- A process exit, callback counter, GPU readback and human observation are
  different evidence. Report exactly which were collected, with hashes.

The kit contains no 0.20 migration script: the historical experiment read
an old Git object unavailable in a fresh clone. Its findings are documented,
not represented as a reproducible build here. Do not invent missing history.

Before submitting changes, run relevant local checks and record what was
actually executed. CI checks cross-compilation and packaging only; hardware
requires a separate run. No account, private SSH wrapper or firmware dump
belongs in a contribution.

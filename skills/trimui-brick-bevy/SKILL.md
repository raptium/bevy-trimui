---
name: trimui-brick-bevy
description: Build, integrate, diagnose and verify native Bevy/wgpu Vulkan applications on TrimUI Brick Pro using the platform kit, local driver workarounds, launcher Apps and pixel evidence. Use for this hardware path, not general Bevy development or original-Brick compatibility claims.
---

# TrimUI Brick Pro native Bevy

Locate the kit checkout from the user's task or workspace. Read its `AGENTS.md`
and `docs/compatibility.json`; when installed separately, resolve these from the
checkout, not from this skill's install directory. If the kit is missing, ask
for its location rather than guessing tools, hosts or a clone URL.

Route by the actual task:

- **First build/run:** read `README.md` and `experiments/bevy-spike/README.md`.
  Use `tools/device/build_bevy_spike.py`; record Rust/Zig versions and the
  resulting `.build.json`. `--offline` requires already cached real crates.
- **Port an application:** read `docs/integration.md`; reuse `boat-render`, the
  image target, readiness gate, display plugin and local patch configuration.
- **Device investigation:** read `docs/device.md`. Query the current firmware,
  actual ELF libraries and GPU instead of substituting an older published SDK.
- **Render/shader/SSH failure:** read `docs/troubleshooting.md`; choose the
  smallest probe that distinguishes the failing layer. Reproduce the actual
  path rather than inferring a GPU failure from a download or transport error.
- **Audio:** read `docs/audio.md` and `tools/device/brick_audio.py`. Keep the
  CPAL patch and ALSA configuration local to the handheld app.
- **Validate/report:** read `docs/evidence.md`. Pair readback with the exact
  binary and extracted scene metadata. State device/firmware/date and what
  remains untested.

Preserve Bevy 0.19.1 / wgpu 29.0.4 unless the user asks for migration. Do not
apply these private surface APIs to a different version without reviewing their
ownership and signature changes. Patch only copied crates under `.scratch/`;
stop at changed anchors and review, never modify the registry or device driver.

For authorized device execution, start offscreen; use a packaged launcher App
for panel acceptance. Keep bounded diagnostic runs and the watchdog. An early
Guide/Start exit may lack a final readback. After the launcher returns, run the
collector and examine its verdict; a zero exit alone is insufficient.

Do not classify a fixed-frame black screenshot before checking pipeline
readiness, or classify SSH presentation flicker before checking launcher display
ownership. Do not use audio callbacks or sink clocks as a substitute for listening.

Return the artifact paths, exact executed command, binary/source/patch hashes,
checks actually performed and remaining hardware gates. Historical reference
results describe the recorded unit; they are not fresh results for this user's
device. Deployment requires the user's intended target and scope; this skill
does not authorize replacing system libraries, stopping services or publishing.

# Bevy on TrimUI Brick Pro

> **Entirely AI-generated experimental software. Use at your own risk. No warranty.**
> The author accepts no liability to the fullest extent permitted by law,
> including if your device is damaged or explodes. Read the [disclaimer](DISCLAIMER.md).

An asset-free starting point for running native Bevy 2D on the TrimUI Brick Pro:
Vulkan rendering, direct panel presentation, evdev exit controls, reproducible
driver workarounds, and tools that verify the pixels actually rendered.

Built for people **and coding agents**. Start with [AGENTS.md](AGENTS.md), or use
the bundled [trimui-brick-bevy skill](skills/trimui-brick-bevy/SKILL.md).
[中文入口](README.zh-CN.md).

## What works

The reference configuration is **Brick Pro, firmware 1.1.2, PowerVR GE8300,
DDK 1.19@6345021, Bevy 0.19.1, wgpu 29.0.4**. In October 2026 we verified
offscreen pixels, launcher presentation and moving Bevy sprites on one physical
unit. The operator confirmed movement, no flicker and return to the launcher.
These are bounded device results, not a performance benchmark or a claim of
support for every firmware, original Brick, Bevy plugin or renderer feature.

The implementation uses `bevy_render` and wgpu's Vulkan backend. A camera renders
into an image; `boat-render` blits it to a `VK_KHR_display` surface after Bevy's
render submission. There is no SDL dependency or desktop compositor on this path.

Included:

- Shared `boat-render` platform crate and a two-sprite animated example, with no
  game art, fonts or audio assets.
- Small wgpu probes, fourteen shader cases, system snapshots and ELF inspection.
- Local-copy wgpu patches, binary/source/patch hashes, launcher packaging and
  run collection with pixel and animation verification.
- Optional CPAL/ALSA build helpers and the documented, physically tested audio
  configuration; the minimal renderer example does not enable audio.

Read [device findings](docs/device.md), [architecture and integration](docs/integration.md),
[troubleshooting](docs/troubleshooting.md) and [evidence boundaries](docs/evidence.md).
The machine-readable [compatibility record](docs/compatibility.json) has dated
results and reproduction commands. Bevy 0.20 is historical research, not the
supported build entrypoint in this kit.

## Build the minimal example

Run commands from the repository root. The tested host is macOS with Rust
1.98.0 and Zig; Linux hosts need their usual build utilities. Python 3.12+ and
Pillow 12+ are needed for launcher packaging (the Rust build itself uses Python's
standard library). Record your actual compiler versions when reproducing;
the extraction host has Zig 0.17.0.

```sh
rustup target add aarch64-unknown-linux-gnu
python3 -m venv .venv
. .venv/bin/activate
python3 -m pip install -r requirements-dev.txt
python3 tools/device/build_bevy_spike.py
python3 tools/device/package_bevy_spike.py --seconds 30
```

The first build downloads real crates. With a populated Cargo cache, use
`build_bevy_spike.py --offline`; `CARGO_HOME` can select that cache. No firmware,
SDK, Vulkan C headers or device snapshot is needed to compile this example.
The optional audio path needs your own device's `libasound.so.2` snapshot.

The binary is at
`.scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/boat-bevy-spike`;
the launcher App is `dist/Apps/boat-bevy-wgpu`. `boat` names in crate, executable,
environment and App paths are preserved from the source project so the existing
tools agree; they do not refer to a game dependency.

## Run and verify on your device

Configure your own SSH alias in `~/.ssh/config`, copy `ssh.local.sh.example` to
`ssh.local.sh`, then `chmod +x ssh.local.sh`. Edit the alias in that private file.
The example selects key authentication before older tools' password-auth default,
and puts forwarded SSH options before the destination.
No host, credentials or device libraries are distributed.

```sh
# Writes only this App directory on your SD card, preserving its save/ history.
python3 tools/device/install_app.py \
  --package dist/Apps/boat-bevy-wgpu --app-name boat-bevy-wgpu
# On the handheld, launch "boat (Bevy WGPU)" from Apps and wait for its return.
python3 tools/device/collect_bevy_spike.py
```

Success requires a clean exit, final surface readback, matching installed binary
hash, correct sprite pixels and animation movement. Expected final marker:
`BEVY_ANIMATION_PASSED`. Guide/Start can exit early, but an early exit may lack
the evidence needed for that verdict. **SSH direct presentation competes with
the launcher and flickers**; use the launcher for visible acceptance.

For a non-presenting GPU check over SSH:

```sh
python3 tools/device/run_wgpu.py \
  --binary .scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/boat-bevy-spike \
  --output .scratch/bevy-spike/runs/first-offscreen
python3 tools/device/verify_bevy_spike_ppm.py \
  .scratch/bevy-spike/runs/first-offscreen/bevy-spike.ppm
```

Use a new output directory on each run. See [the example](experiments/bevy-spike/README.md)
and [shader probes](experiments/wgpu-probe/README.md) for more commands.

## Use with an AI agent

`AGENTS.md` is the repository entrypoint. To install the skill in another checkout,
copy `skills/trimui-brick-bevy/` to that agent's skill directory (for example,
`~/.codex/skills/trimui-brick-bevy/`). Tell the agent where this checkout is;
the skill intentionally resolves documentation from the checkout rather than
from a developer's machine. A useful first request is:

> Use $trimui-brick-bevy to build the minimal example in this checkout. Report
> compiler versions, binary/source/patch hashes and what still needs device testing.

## License and contributions

Original kit code, documentation and Skill: [MIT](LICENSE). Upstream crates and
generated patched copies retain their upstream licenses; see [NOTICE](NOTICE.md).
Device firmware, proprietary libraries, game assets, secrets and original Git
history are not included. A snapshot tool does not grant redistribution rights
to the resulting firmware dump.

For fixes and reproduction reports, see [CONTRIBUTING.md](CONTRIBUTING.md).
The kit was extracted from Boat; [SOURCE-MANIFEST.json](SOURCE-MANIFEST.json)
records the source revision and exact exported file hashes. It is a source
provenance record, not a device test result. This project is not affiliated with
TrimUI, Bevy or the wgpu project.

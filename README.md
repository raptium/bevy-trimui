# Bevy TRIMUI

> **Entirely AI-generated experimental software. Use at your own risk. No warranty.**
> The author accepts no liability to the fullest extent permitted by law,
> including if your device is damaged or explodes. Read the [disclaimer](DISCLAIMER.md).

Native Bevy platform tools for TRIMUI handhelds, with an asset-free 2D starting point:
Vulkan rendering, direct panel presentation, evdev exit controls, reproducible
driver workarounds, and tools that verify the pixels actually rendered.

The reference device is a **TRIMUI Brick Pro**, the only model currently tested.
The rendering and build approach may transfer to other TRIMUI devices, but their
firmware, GPU drivers, display ownership and input/audio paths must be checked
on the actual hardware before claiming support. The project name describes the
platform family; it is not a compatibility guarantee.

Built for people **and coding agents**. Start with [AGENTS.md](AGENTS.md), or use
the bundled [bevy-trimui skill](skills/bevy-trimui/SKILL.md).
[中文入口](README.zh-CN.md).

## What works

The reference configuration is **Brick Pro, firmware 1.1.2, PowerVR GE8300,
DDK 1.19@6345021, Bevy 0.19.1, wgpu 29.0.4**. In October 2026 we verified
offscreen pixels, launcher presentation and moving Bevy sprites on one physical
unit. The operator confirmed movement, no flicker and return to the launcher.
These are bounded device results, not a performance benchmark or a claim of
support for every firmware, original Brick, Bevy plugin or renderer feature.

The implementation uses `bevy_render` and wgpu's Vulkan backend. A camera renders
into an image; `bevy-trimui` blits it to a `VK_KHR_display` surface after Bevy's
render submission. There is no SDL dependency or desktop compositor on this path.

Included:

- Shared `bevy-trimui` platform crate and a two-sprite animated example, with no
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

Run commands from the repository root. Install Rust (tested with 1.98.0),
[uv](https://docs.astral.sh/uv/getting-started/installation/) and a working SSH
client for device access. `uv run` manages Python 3.12+ and installs the locked
Pillow and Zig 0.17.0 tools from `pyproject.toml` / `uv.lock`. No manual Python
environment activation or separate Zig installation is required. Linux hosts
also need their usual build utilities.

```sh
rustup target add aarch64-unknown-linux-gnu
uv run tools/device/build_bevy_spike.py
uv run tools/device/package_bevy_spike.py --seconds 30
```

The first build downloads real crates. With a populated Cargo cache, use
`build_bevy_spike.py --offline`; `CARGO_HOME` can select that cache. No firmware,
SDK, Vulkan C headers or device snapshot is needed to compile this example.
The optional audio path needs your own device's `libasound.so.2` snapshot.

The binary is at
`.scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/bevy-trimui-demo`;
the launcher App is `dist/Apps/bevy-trimui-demo`. The reusable platform crate is
`bevy-trimui`; the standalone examples and App paths use the Bevy TRIMUI name.
Builds use the uv-locked Zig through `cross/zig`. To populate the Python cache
in advance, use `uv sync --locked`; `uv run --offline ...` also requires a
populated Python cache, separate from Cargo's `--offline` option.

## Run and verify on your device

Configure your own SSH alias in `~/.ssh/config`, copy `ssh.local.sh.example` to
`ssh.local.sh`, then `chmod +x ssh.local.sh`. Edit the alias in that private file.
The example selects key authentication before older tools' password-auth default,
and puts forwarded SSH options before the destination.
No host, credentials or device libraries are distributed.
Alternatively, set `BEVY_TRIMUI_SSH_WRAPPER` to the absolute path of your own private
wrapper. Input probing can be overridden with `BEVY_TRIMUI_EVDEV`; see
[integration](docs/integration.md).

```sh
# Writes only this App directory on your SD card, preserving its save/ history.
uv run tools/device/install_app.py \
  --package dist/Apps/bevy-trimui-demo --app-name bevy-trimui-demo
# On the handheld, launch "Bevy TRIMUI Demo" from Apps and wait for its return.
uv run tools/device/collect_bevy_spike.py
```

Success requires a clean exit, final surface readback, matching installed binary
hash, correct sprite pixels and animation movement. Expected final marker:
`BEVY_ANIMATION_PASSED`. Guide/Start can exit early, but an early exit may lack
the evidence needed for that verdict. **SSH direct presentation competes with
the launcher and flickers**; use the launcher for visible acceptance.

For a non-presenting GPU check over SSH:

```sh
uv run tools/device/run_wgpu.py \
  --binary .scratch/bevy-spike/target/aarch64-unknown-linux-gnu/release/bevy-trimui-demo \
  --output .scratch/bevy-spike/runs/first-offscreen
uv run tools/device/verify_bevy_spike_ppm.py \
  .scratch/bevy-spike/runs/first-offscreen/bevy-spike.ppm
```

Use a new output directory on each run. See [the example](experiments/bevy-spike/README.md)
and [shader probes](experiments/wgpu-probe/README.md) for more commands.

## Use with an AI agent

`AGENTS.md` is the repository entrypoint. To install the skill in another checkout,
copy `skills/bevy-trimui/` to that agent's skill directory (for example,
`~/.codex/skills/bevy-trimui/`). Tell the agent where this checkout is;
the skill intentionally resolves documentation from the checkout rather than
from a developer's machine. A useful first request is:

> Use $bevy-trimui to build the minimal example in this checkout. Report
> compiler versions, binary/source/patch hashes and what still needs device testing.

## License and contributions

Original kit code, documentation and Skill: [MIT](LICENSE). Upstream crates and
generated patched copies retain their upstream licenses; see [NOTICE](NOTICE.md).
Device firmware, proprietary libraries, game assets, secrets and original Git
history are not included. A snapshot tool does not grant redistribution rights
to the resulting firmware dump.

For fixes and reproduction reports, see [CONTRIBUTING.md](CONTRIBUTING.md).
For consuming this as a separate Git dependency and promoting improvements from
a private application, see [development workflow](docs/development.md).
The kit was extracted from an earlier private application;
[SOURCE-MANIFEST.json](SOURCE-MANIFEST.json) records its original source revision
and a refreshed inventory of this kit's current file hashes. It is a source
provenance record, not a device test result. This project is not affiliated with
TrimUI, Bevy or the wgpu project.

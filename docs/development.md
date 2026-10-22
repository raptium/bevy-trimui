# Public platform, private application

Bevy TRIMUI has its own Git repository:
[raptium/bevy-trimui](https://github.com/raptium/bevy-trimui).
Consumers keep their own source and assets private while pinning a validated
platform revision. This repository contains reusable platform code, minimal
examples, diagnostics, documentation and the Skill, without consumer game code.

## Consume the platform

```sh
git submodule add https://github.com/raptium/bevy-trimui.git platform/bevy-trimui
git submodule update --init --recursive
```

The application's Cargo workspace excludes the nested platform workspace:

```toml
# Application workspace Cargo.toml
[workspace]
members = ["game"]
exclude = ["platform/bevy-trimui"]

# game/Cargo.toml
[dependencies]
bevy-trimui = { path = "../platform/bevy-trimui/crates/bevy-trimui" }

[features]
handheld = ["bevy-trimui/direct-display"]
```

Use `bevy_trimui` in Rust. Call the platform's `prepare_wgpu_display.py` with
the application's own vendor output directory and import the audio helpers
when needed. Linker/C shims under `cross/` use the current uv environment's Zig.
The consumer owns its content, packaging, saves and private SSH credentials.

The public installer accepts `--executable NAME` for an application's own binary,
in addition to `--package` and `--app-name`. `BEVY_TRIMUI_SSH_WRAPPER` selects a
private wrapper outside this repository; never copy credentials into the submodule.

## Iterate and promote tested changes

1. Develop shared code inside the platform checkout on a branch. A newly checked
   out submodule may have a detached HEAD; create a branch before editing.
2. Run platform tests and small builds with `uv run`, then test the actual
   application. Record build, GPU and physical-device checks separately.
3. Commit and publish reusable platform changes from this repository within the
   user's authorized scope. Do not copy a whole application tree into it.
4. In the private application repository, stage the submodule pointer and commit
   the validated revision alongside the corresponding application changes.

For a fresh private application checkout use `git clone --recurse-submodules`
or `git submodule update --init --recursive`. Editing or pulling inside the
platform does not change the parent's recorded revision until it stages that
pointer. Keep the renderer, driver patches and device audio configuration here
rather than maintaining edited copies in each consumer.

## Additional TRIMUI hardware

The approach may transfer to other models, but only Brick Pro has a recorded
device result. First inspect each target's actual firmware, ABI, GPU/DDK,
display extensions, input events and audio capabilities. Reproduce the small
offscreen and launcher examples before promoting support. A shared brand does
not prove identical hardware or driver behavior.

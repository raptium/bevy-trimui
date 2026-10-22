# Integrating a Bevy application

## Rendering boundary

`crates/trimui-brick-render` is the shared platform layer. The minimal example in
`experiments/bevy-spike/src/main.rs` exercises it without game assets.

```text
Bevy main world: Camera2d + Sprite + Transform
    -> extraction / native Bevy render graph
    -> 1024x768 Rgba8UnormSrgb image target
    -> render_system completes GPU submission
    -> trimui-brick-render fullscreen blit
    -> imported VK_KHR_display surface / swapchain / present
```

`add_offscreen_target` installs the image and `RenderReadiness`. Assign its handle
to the camera's `RenderTarget::Image`, with `Msaa::Off`. For presentation add
`DisplayPresentPlugin::new(target, frame_limit, seconds, readback_path)` after
the render app exists. The example uses `ScheduleRunnerPlugin`, without winit.
For your own interactive application replace the diagnostic run limits with
its intended lifecycle and retain an appropriate stall watchdog and exit input.

The target uses `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC`. The blit runs
after `render_system` and readiness recording in the render schedule. Rendering
and presentation objects stay in the render world; completion/failure crosses
back through synchronized flags. Scene pose metadata is extracted with the
rendered frame so a readback can be checked against the corresponding position.

Readiness requires at least one compiled pipeline, no waiting pipelines and two
consecutive ready render frames. Retryable shader loading is allowed; actual
shader compilation errors fail. A screenshot requested on an arbitrary early
frame may be black even when later frames are correct.

## Dependencies and patches

Keep Bevy `=0.19.1`, wgpu/hal/core `29.0.4` and ash `=0.38.0` aligned with the
lockfiles. Enable `trimui-brick-render/brick-display` only for the handheld path. The
crate currently inherits Bevy from the root workspace; when moving it, carry
over that dependency declaration or replace it with the same explicit features.
`publish = false` reflects that it uses a locally patched wgpu API; this is a
source integration kit rather than a published crates.io plugin.

`tools/device/build_bevy_spike.py` fetches real crates, copies three wgpu crates
into `.scratch/wgpu-probe/vendor`, applies exact-anchor changes and passes Cargo
`--config patch.crates-io.<crate>.path=...` overrides. Reuse this configuration
for an application manifest. A plain build with `brick-display` cannot find
`import_display_surface` / `create_surface_from_hal` in unpatched wgpu.

The two independent modifications are:

1. **PowerVR submit compatibility.** The observed driver crashed on the second
   submission with mixed timeline/binary semaphores. The local HAL copy pads
   ignored binary-semaphore value entries when timeline values are present.
   [Khronos specifies that binary entries' values are ignored](https://docs.vulkan.org/refpages/latest/refpages/source/VkTimelineSemaphoreSubmitInfo.html).
   The driver-specific workaround was established experimentally; that spec
   alone does not establish the crash cause on other driver versions.
2. **Owned surface import.** A `VK_KHR_display` surface is created through the
   exact Vulkan instance used by wgpu, then transferred through HAL/core/wgpu.
   Ownership transfers once. Do not create it from another instance or destroy
   it separately afterward. Keep the native display valid for the surface lifetime.

Patch generators check unique source anchors and save diffs and hashes. If an
anchor changes, inspect the new upstream source and revalidate the behavior;
do not broaden string replacements to hide the mismatch. Do not patch a shared
Cargo registry or replace device libraries.

## Cross compilation

`.cargo/config.toml` selects `cross/zig-cc-wrapper` for
`aarch64-unknown-linux-gnu`. Zig targets glibc 2.25 for a conservative link ABI;
the observed device actually has 2.33. The linker embeds
`RUNPATH=/rom/usr/trimui/lib`. The C shim preserves argument boundaries while
translating the Rust target triple and filtering the unsupported Cortex-A53
erratum flag. `uv run` supplies the Python environment; `cross/zig` runs its
locked `ziglang` dependency, including when Cargo invokes the linker shims.

No SDL SDK, Vulkan C headers or device library upload is required for the
renderer example. For optional audio, use the device ABI as described in
[audio.md](audio.md). Record toolchain versions, the lockfiles, generated
patches and the `.build.json` alongside each binary.

## Input

The example's `src/evdev.rs` probes `/dev/input/event0..11` by capability rather
than assuming event0. `BRICK_EVDEV` overrides the path. It decodes 24-byte
little-endian `input_event` records for 64-bit Linux. Guide (`0x13c`) and Start
(`0x13b`) exit on a press. This small reader is an exit mechanism, not a complete
controller abstraction or reconnection layer.

The original game experiments found D-pad direction in `EV_ABS` `ABS_HAT0X/Y`, requiring
button edges for menu navigation as well as held movement. The physical printed
A/B mapping used East=A/confirm and South=B/back; do not assume desktop
South=confirm matches the labels. Recheck raw input and the actual controls on
your device. `decode_evdev.py` can inspect captured binary input records.

## Version migration boundary

A separate 2026-10-09 Bevy 0.20 / wgpu 30.0.1 experiment presented moving sprites.
It needed official `wgpu/drm` to request `VK_KHR_display`; without it the app
reported `VK_KHR_display was not enabled on this instance` and exited cleanly.
Other changes included explicit `#[extract_app(RenderApp)]`, renderer wrapper
access, `Queue::present`, fallible mapped ranges and the HAL surface signature.

This kit intentionally ships the 0.19 baseline only. The historical 0.20 script
read a fixed source tree from the original private Git history and is not included as a
portable command. Migrate from the included source in a separate branch, with
new lockfiles, source review and device validation. Historical plugin-version
availability is not a current dependency recommendation.

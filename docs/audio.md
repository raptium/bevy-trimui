# Optional Brick Pro audio

The minimal sprite example has no audio dependency. This kit includes
`tools/device/brick_audio.py` and `alsa-brick.conf` so an application using Bevy
audio can reproduce the platform configuration without importing the original game.

The reference game used Bevy 0.19.1, rodio 0.22.2 and CPAL 0.17.3. On 2026-10-17
the operator confirmed music, footsteps, menu sounds and room transitions through
the Brick Pro speaker. The accepted game binary hash was
`c445ad7788d32f59646289f878eb8cd313fca4e9d3b610f9d1c3423bcd6944bc`.
That game and its audio assets are not distributed in this renderer kit. This
is historical integration evidence, not an audio smoke test executed by the
included minimal example.

## What changed after a successful link

The firmware's default asym/plug/softvol/dmix chain caused CPAL to submit one
7,524-sample chunk and then stall. A direct hardware PCM avoided that stall.
However, the codec-advertised S24_LE / 44,100 Hz path still produced no audible
sound even with advancing callbacks, PCM pointers and sink clocks.

The firmware UI used **stereo S16_LE / 48,000 Hz**. A direct tone in that format
was heard, then the full game's speaker output was confirmed. The local CPAL
patch explicitly selects that supported format and fails if unavailable; it
does not fall back to the observed silent 24-bit mode. The evidence narrows the
compatibility issue to the format path, not a proven bug in a specific library.

The unpatched hardware default selected a 16,384-frame period and roughly
743 ms total buffering. The local patch requests a 1,024-frame period and
2,048-frame buffer, observed at 48 kHz (about 42.7 ms total). This is an ALSA
buffer duration, not a measured end-to-end input-to-speaker latency.

## Apply to an audio-enabled app

1. Enable Bevy audio and the chosen decoder in your application, and fetch its
   real target dependencies. Keep CPAL 0.17.3 pinned for the provided patch.
2. Collect your device snapshot with `snapshot.py`. In a Python build entrypoint,
   import `prepare_alsa` and `prepare_cpal` from the sibling `brick_audio` module:

   ```python
   # In tools/device/<your-builder>.py, with an existing env and output directory:
   env.update(prepare_alsa(output, snapshot))
   prepare_cpal(vendor, cargo_home)
   # Add to the same cargo build that selects the handheld application:
   cmd += ["--config", f'patch.crates-io.cpal.path="{vendor / "cpal"}"']
   ```

   `prepare_alsa` exposes `snapshot/rootfs/usr/lib/libasound.so.2` as a local
   linker input plus `alsa.pc`. Set its returned target-specific
   `PKG_CONFIG_ALLOW_CROSS`, `PKG_CONFIG_LIBDIR` and `PKG_CONFIG_PATH` variables
   for the build. The current alsa-sys bindings are pregenerated, so this path
   does not need ALSA C headers. It must not pick host macOS/Linux ALSA metadata.
3. Copy `alsa-brick.conf` into the App. Export `ALSA_CONFIG_PATH` to its absolute
   path in `launch.sh`. Do **not** include the system `alsa.conf`: a deferred hook
   reimports `/etc/asound.conf` and can restore the problematic default chain.
4. Keep volume under firmware control and application mixing in rodio. Do not
   change `/etc/asound.conf` or any system audio library.
5. Save binary, CPAL patch, config and device library hashes. Test a tone, then
   the actual application through the launcher, with deliberate listening and
   clean return. Check both device mute/volume and the app's volume setting.

The helper can prepare inputs independently:

```sh
uv run tools/device/brick_audio.py \
  --snapshot .device-snapshots/brick-pro-YOUR-TIMESTAMP \
  --output .scratch/my-app
```

It prints the pkg-config environment and CPAL Cargo override; it does not
compile your app or change its launcher. Long runs, headphones and Bluetooth
were not established by the reference speaker test.

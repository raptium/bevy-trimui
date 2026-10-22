# Troubleshooting by failing layer

| Symptom | Next discriminating check |
| --- | --- |
| Missing cached wgpu/CPAL or offline crate error | Fetch real target crates online; offline is only a cache mode. Check `CARGO_HOME`. |
| `import_display_surface` / `create_surface_from_hal` missing | Use the build script's three aligned local wgpu overrides. |
| Changed patch anchor | Review the exact upstream crate version; do not patch the registry or guess a replacement. |
| ELF links but app will not load | Inspect AArch64 machine, NEEDED, RUNPATH and actual device SONAMEs. |
| Early black screenshot, exit 0 | Check pipeline readiness and retryable asset/shader state before declaring GPU failure. |
| SSH-presented frame flickers | Launch the packaged App from Apps so launcher gives up display ownership. |
| Marker below center | Inspect the blit UV Y mapping and render target orientation. |
| Correct shapes but dark colors | Check chosen surface format (`Bgra8UnormSrgb` expected in the reference) and color conversion. |
| Animation present count grows but position does not | Match each readback to its extracted pose; count alone is insufficient. |
| Collector reports missing final image | Check early pad exit, watchdog, render/readback errors and clean auto-return. |
| SSH timeout or partial archive | Transmission plus execution share a timeout. No device verdict was established. |
| No menu navigation despite movement | D-pad hats need press edges, not only analog axes; inspect raw evdev codes. |
| Audio sink exists but silent | Check actual PCM callbacks, then verified format and human listening; see audio.md. |

Use unique output directories. The collector refuses unfinished runs and checks
the installed binary against that run's metadata; do not mix images from an
older run with current logs. Its animation gate expects at least four sampled
poses, strictly increasing scene frames and at least 128 pixels of position
spread. A sampling cadence aligned with an oscillation can under-sample motion;
check the paired images rather than reclassifying it as a GPU failure. The
current sample schedule includes intermediate phases.

## Shader cases

The 2026-10-08 matrix on firmware 1.1.2 / DDK 1.19@6345021 found the following.
All cases draw the same triangle. The diagnostic checks 62,014 non-edge pixels
per frame rather than trusting a successful API call.

| `--vertex-*` | Recorded result |
| --- | --- |
| branch | Pass |
| array (`let array<vec2,3>` with dynamic index) | Fail |
| array-store (declare, write elements, index) | Pass |
| array-assign (declare, assign whole array, index) | Pass |
| array-init (`var a = array(...)`, dynamic index) | Fail |
| array-const (constant index) | Pass |
| array-f32 | Fail |
| array-vec3 | Pass |
| array-vec4 | Pass |
| matrix | Pass |
| vec (dynamic vector extraction) | Fail |
| select | Pass |
| uniform-array | Pass |
| storage-array | Pass |

The failures correlate with function-local composite values and dynamic
extraction, not all arrays or all dynamic indexing. The tested SSBO form used
by mesh-instance access passed. Bevy 0.19.1 sprites used arithmetic vertex
construction and were directly verified. No general Naga lowering fix was
implemented; each new shader path needs its own device check.

Possible source workarounds established in these cases: arithmetic, select or
switch for small fixed lookups; UBO/SSBO tables; declare-then-assign or individual
writes for local arrays. Do not extrapolate one pass to all generated SPIR-V.

```sh
cargo fetch --manifest-path experiments/wgpu-probe/Cargo.toml \
  --target aarch64-unknown-linux-gnu
uv run tools/device/build_wgpu.py
uv run tools/device/run_wgpu.py -- --vertex-array --frames 2
# Run separately; the previous case is an intentionally failing comparison.
uv run tools/device/run_wgpu.py -- --vertex-array-store --frames 2
```

`build_wgpu.py --stock` omits the submit workaround for a deliberate driver
comparison; it may crash. It is not the normal build. MSAA stays off; this
matrix establishes nothing about 3D/OIT/compute or long-term driver stability.

# wgpu probes

An independent wgpu 29.0.4 workspace for separating shader/driver/display failures
from Bevy application behavior. It uses the same local HAL and surface-import
patches as the Bevy example.

```sh
cargo fetch --manifest-path experiments/wgpu-probe/Cargo.toml \
  --target aarch64-unknown-linux-gnu
uv run tools/device/build_wgpu.py
uv run tools/device/run_wgpu.py -- --frames 120

# Query display capabilities (no visible drawing).
uv run tools/device/build_wgpu.py --display
uv run tools/device/run_wgpu.py -- --display-info
# Direct presentation is diagnostic: it competes with launcher and may flicker.
uv run tools/device/run_wgpu.py -- --present --frames 120
```

`build_wgpu.py` builds offline after the explicit fetch. `--stock` is a deliberate
comparison without the HAL workaround and may reproduce the driver crash; it
cannot be combined with `--display`.

The [shader matrix](../../docs/troubleshooting.md) lists all fourteen
`--vertex-*` variants, expected historical outcomes and source workarounds.
The probe compares triangle pixels, saves logs/readbacks, and records binary,
source and patch hashes. `run_wgpu.py` gzip-uploads to a temporary device path,
enforces a 45-second remote execution limit, retrieves evidence and removes
the device temporary directory. Its `--timeout` covers transfer plus execution;
increasing that option does not extend the remote execution limit.

An explicit `--output` must name a new directory. The default run output is
`.scratch/wgpu-probe/runs/<timestamp>`. Use launcher packaging of the Bevy scene
for human panel acceptance; direct presentation/readback does not establish an
exclusive, flicker-free display session.

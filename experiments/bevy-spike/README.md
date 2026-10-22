# Minimal Bevy scene

Two colored sprites on a 1024×768 image target. Default mode captures a static
offscreen PPM; `--display --animate --seconds 30` presents a moving orange square
with a fixed blue orientation marker. The shared `trimui-brick-render` crate owns the
readiness and display logic. There are no external game assets or audio devices.

From the repository root:

```sh
uv run tools/device/build_bevy_spike.py
uv run tools/device/package_bevy_spike.py --seconds 30
uv run tools/device/install_app.py \
  --package dist/Apps/brick-bevy-demo --app-name brick-bevy-demo
# Select "Brick Pro (Bevy Demo)" in Apps, then wait for automatic return.
uv run tools/device/collect_bevy_spike.py
```

Guide/Start exits early; the process watchdog bounds a wedged diagnostic run.
Evidence is stored in the App's `save/run-*/` and collected to
`.scratch/bevy-spike/runs/`. The collector checks binary hashes, exit code,
surface pixels and multiple extracted poses. `BEVY_ANIMATION_PASSED` is the
animation verdict, not a performance claim.

For offscreen execution and verification, use the explicit binary and new output
path shown in the [root README](../../README.md). A static capture expects
65,536 orange pixels at x=384..639/y=256..511 and 4,096 blue pixels at
x=480..543/y=152..215. Animation uses each readback's adjacent JSON center.

`--frames N` in offscreen mode is minimum warmup, not a substitute for readiness.
Display defaults to 600 frames if neither a duration nor frame count is given.
With `--seconds`, elapsed time controls completion unless an explicit frame
limit is also set. Keep diagnostic runs finite. See
[integration](../../docs/integration.md) for adapting this to your own app.

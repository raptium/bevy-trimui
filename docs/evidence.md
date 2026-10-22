# Evidence and acceptance

## Historical reference results

These statements were transcribed from Boat's dated research records, not
rerun during extraction. Raw firmware dumps and full device run archives stay
private; this public kit ships the source, procedures and selected result
summaries. Public consumers can reproduce the experiments but cannot audit all
original raw runs from this source package alone.

| Date | Result | What it establishes |
| --- | --- | --- |
| 2026-10-08 | Vulkan 1.3.225 and GE8300 enumeration; runtime files hashed | Device/library identity, not drawing |
| 2026-10-08 | wgpu 29.0.4 offscreen triangle and presentation readback passed with local patches | Required GPU submission and raster path |
| 2026-10-08 | Fourteen shader cases, mixed outcomes | Specific source forms on the recorded driver |
| 2026-10-09 | Bevy 0.19.1 offscreen 1024×768 sprite/marker pixel checks passed | Native Bevy sprite pipeline |
| 2026-10-09 | Static launcher run: 1,208 presents in 30 s, matching final/offscreen pixels, operator confirmed no flicker/return | Static panel path on this unit |
| 2026-10-09 | Animated launcher run: 1,228 presents in 30 s, sampled centers 376/652/372/484, operator confirmed moving/no flicker/return | Transform update through render extraction to panel |
| 2026-10-09 | Separate 0.20 / wgpu 30.0.1 launcher test passed after enabling drm | A migration was feasible then, not a build supplied here |
| 2026-10-17 | Game's S16_LE / 48 kHz music, footsteps and menu output heard by operator | Speaker playback in the separate integration |

Reference animation binary SHA-256:
`9466b9f91b6b39477084ef119233a85a986db2e5c6e7fe117befb88e49ca3621`.
The current source has evolved since that run (including sampling and shared
crate extraction); rebuilding it need not reproduce that binary hash.
The render runs recorded healthy GPU status and no increase in the inspected
error counters. Counts with synchronous waits/readbacks are not FPS benchmarks.

## Accepting a new build

Keep separate verdicts for each gate:

1. **Source/build:** exact lockfiles and compiler versions; generated patch
   hashes; successful target build; `.build.json` binary hash.
2. **ELF/load:** AArch64 (ELF machine 183), expected NEEDED/RUNPATH and actual
   library availability. A link result alone proves no GPU behavior.
3. **Offscreen GPU:** clean exit and actual PPM pixels from the same binary.
   Wait for readiness, check marker orientation and shape bounds.
4. **Launcher presentation:** completed run, installed binary hash matching
   per-run metadata, final surface readback, paired animation poses and clean
   return. Run `collect_bevy_spike.py`; investigate any failed evidence gate.
5. **Human panel/control/audio:** operator confirms movement and flicker,
   physical input, return and (when applicable) actual audible output. An
   image readback does not observe the physical panel or speaker.

Report a gate as passed only when that evidence exists; otherwise say not run
or insufficient evidence. A collector transport failure is not a GPU verdict.
Save the exact command, run identifier and artifact paths with the summary.

The kit has no validated claim for the original Brick, other firmware/DDK
versions, arbitrary plugins, advanced text/UI/3D/postprocessing, suspend/resume,
reconnection, power loss, long-duration stability, headphones or Bluetooth.

## Sanitized contributions

Share a compact per-run record with hashes, firmware/DDK, exit status and pixel
verdict. Remove host/user paths and addresses from build commands before posting.
Never include private wrappers, credentials, user saves or proprietary runtime
libraries. Do not silently change an old run record when reorganizing sources;
new paths or rebuilt binaries need a new provenance record.

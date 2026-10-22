#!/usr/bin/env python3
"""Prepare app-local ALSA linkage and the pinned Brick Pro CPAL workaround."""
from __future__ import annotations

import argparse
import difflib
import json
import os
from pathlib import Path
import shutil


def prepare_alsa(output: Path, snapshot: Path) -> dict[str, str]:
    """Expose the device's real ALSA ABI to pkg-config and the cross linker."""
    library = snapshot / "rootfs/usr/lib/libasound.so.2"
    if not library.is_file():
        raise SystemExit(f"Device ALSA library missing: {library}; pass --snapshot with a device snapshot")
    libdir = output / "alsa/lib"
    pcdir = output / "alsa/pkgconfig"
    libdir.mkdir(parents=True, exist_ok=True)
    pcdir.mkdir(parents=True, exist_ok=True)
    link = libdir / "libasound.so"
    link.unlink(missing_ok=True)
    link.symlink_to(library.resolve())
    # alsa-sys uses pregenerated bindings; no development headers are needed.
    (pcdir / "alsa.pc").write_text(
        f"libdir={libdir}\n\nName: alsa\nDescription: Device ALSA runtime\n"
        "Version: 1.0.0\nLibs: -L${libdir} -lasound\n"
    )
    return {"PKG_CONFIG_ALLOW_CROSS_aarch64_unknown_linux_gnu": "1",
            "PKG_CONFIG_LIBDIR_aarch64_unknown_linux_gnu": str(pcdir),
            "PKG_CONFIG_PATH_aarch64_unknown_linux_gnu": str(pcdir)}


def prepare_cpal(vendor: Path, cargo_home: Path) -> None:
    """Use the audibly verified PCM format and latency in the Brick build."""
    matches = sorted((cargo_home / "registry/src").glob("*/cpal-0.17.3"))
    if len(matches) != 1:
        raise SystemExit("Expected one cached cpal 0.17.3; revalidate the ALSA period patch when upgrading")
    destination = vendor / "cpal"
    shutil.copytree(matches[0], destination, dirs_exist_ok=True)
    relative = "src/host/alsa/mod.rs"
    path = destination / relative
    original = path.read_text()
    anchor = "    // Apply hardware parameters\n    pcm_handle.hw_params(&hw_params)?;"
    replacement = """    // The Brick codec otherwise chooses a 16384-frame period (~372ms).
    // This crate copy is used only by the isolated handheld build. Rodio owns
    // the game mixer and a double 1024-frame period keeps output responsive.
    if config.buffer_size == BufferSize::Default {
        hw_params.set_period_size_near(1024, alsa::ValueOr::Nearest)?;
        hw_params.set_buffer_size_near(2048)?;
    }

    // Apply hardware parameters
    pcm_handle.hw_params(&hw_params)?;"""
    if original.count(anchor) != 1:
        raise SystemExit("CPAL ALSA source changed; revalidate the hardware buffer patch")
    changed = original.replace(anchor, replacement)
    format_anchor = "        formats.sort_by(|a, b| a.cmp_default_heuristics(b));"
    format_replacement = """        // The Brick codec advertises S24_LE, but physical listening tests
        // were silent in that mode. Stereo S16_LE at 48kHz is audible and is
        // also the firmware UI's format. Never silently fall back to S24_LE.
        if matches!(stream_t, alsa::Direction::Playback) {
            return formats.iter()
                .find(|f| f.sample_format() == SampleFormat::I16
                    && f.channels() == 2
                    && f.min_sample_rate() <= 48_000
                    && f.max_sample_rate() >= 48_000)
                .map(|f| f.clone().with_sample_rate(48_000))
                .ok_or(DefaultStreamConfigError::StreamTypeNotSupported);
        }

        formats.sort_by(|a, b| a.cmp_default_heuristics(b));"""
    if changed.count(format_anchor) != 1:
        raise SystemExit("CPAL ALSA format selection changed; revalidate the Brick PCM format")
    changed = changed.replace(format_anchor, format_replacement)
    path.write_text(changed)
    patch = "".join(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),
                                       fromfile="cpal/" + relative, tofile="cpal/" + relative))
    (vendor / "cpal-alsa-period.patch").write_text(patch)



def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cargo-home", type=Path,
                        default=Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))))
    args = parser.parse_args()
    output = args.output.resolve()
    env = prepare_alsa(output, args.snapshot.resolve())
    vendor = output / "vendor"
    prepare_cpal(vendor, args.cargo_home.resolve())
    print(json.dumps({"environment": env,
                      "cargo_config": f'patch.crates-io.cpal.path="{vendor / "cpal"}"'}, indent=2))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Verify a bevy-spike PPM: offscreen screenshot or presented-surface readback.

Expected scene (experiments/bevy-spike/src/main.rs): a 256x256 orange sprite centered
on a 1024x768 target, plus a 64x64 blue marker 200px above center. The marker
exists so a vertical flip anywhere in the chain fails loudly: a flipped marker
lands below center.

Both bevy-spike.ppm (main-world screenshot) and display.ppm (read back from the
presented surface) use this check. Pass --sprite/--marker to override sizes.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WIDTH = 1024
HEIGHT = 768
SPRITE = 256
MARKER = 64
MARKER_OFFSET_Y = 200  # world units above center


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("ppm", type=Path)
    ap.add_argument("--sprite", type=int, default=SPRITE)
    ap.add_argument("--marker", type=int, default=MARKER)
    ap.add_argument("--marker-offset-y", type=int, default=MARKER_OFFSET_Y)
    ap.add_argument("--sprite-center-x", type=int,
                    help="defaults to adjacent readback JSON when present, otherwise image center")
    args = ap.parse_args()

    data = args.ppm.read_bytes()
    header = data.split(b"\n", 3)
    if len(header) != 4 or header[0] != b"P6":
        raise SystemExit(f"{args.ppm}: not a binary PPM")
    width, height = map(int, header[1].split())
    pixels = header[3]
    if (width, height) != (WIDTH, HEIGHT) or len(pixels) != WIDTH * HEIGHT * 3:
        raise SystemExit(f"unexpected image {width}x{height} bytes={len(pixels)}")

    def pixel(x: int, y: int) -> tuple[int, int, int]:
        offset = (y * width + x) * 3
        return tuple(pixels[offset:offset + 3])  # type: ignore[return-value]

    def is_sprite(p: tuple[int, int, int]) -> bool:
        return p[0] > 200 and 80 < p[1] < 180 and p[2] < 60

    def is_marker(p: tuple[int, int, int]) -> bool:
        return p[0] < 80 and 80 < p[1] < 180 and p[2] > 200

    # One pass over the pixel buffer; per-pixel helper calls are too slow here.
    sprite_pixels: list[tuple[int, int]] = []
    marker_pixels: list[tuple[int, int]] = []
    for index in range(0, len(pixels), 3):
        red, green, blue = pixels[index], pixels[index + 1], pixels[index + 2]
        if red > 200 and 80 < green < 180 and blue < 60:
            sprite_pixels.append(((index // 3) % width, (index // 3) // width))
        elif red < 80 and 80 < green < 180 and blue > 200:
            marker_pixels.append(((index // 3) % width, (index // 3) // width))

    center = pixel(width // 2, height // 2)
    corners = [pixel(2, 2), pixel(width - 3, 2), pixel(2, height - 3), pixel(width - 3, height - 3)]

    sprite_center_x = args.sprite_center_x
    metadata = args.ppm.with_suffix(".json")
    if sprite_center_x is None and metadata.is_file():
        sprite_center_x = json.loads(metadata.read_text())["sprite_center_x"]
    if sprite_center_x is None:
        sprite_center_x = width // 2
    if not isinstance(sprite_center_x, int) or not args.sprite // 2 <= sprite_center_x <= width - args.sprite // 2:
        raise SystemExit("invalid sprite center in readback metadata")
    expected_sprite = sprite_center_x, height // 2
    center = pixel(*expected_sprite)
    expected_marker = width // 2, height // 2 - args.marker_offset_y

    def expect_bbox(name: str, points: list[tuple[int, int]], size: int, at: tuple[int, int]) -> list[str]:
        cx, cy = at
        if len(points) != size * size:
            return [f"{name} pixel count {len(points)} != {size * size}"]
        xs = [p[0] for p in points]
        ys = [p[1] for p in points]
        expected_x = (cx - size // 2, cx + size // 2 - 1)
        expected_y = (cy - size // 2, cy + size // 2 - 1)
        if (min(xs), max(xs)) != expected_x or (min(ys), max(ys)) != expected_y:
            return [f"{name} bbox x={min(xs)}..{max(xs)} y={min(ys)}..{max(ys)} "
                    f"!= x={expected_x} y={expected_y}"]
        return []

    problems = []
    if not is_sprite(center):
        problems.append(f"center {center} is not the sprite color")
    marker_center = pixel(expected_marker[0], expected_marker[1])
    if not is_marker(marker_center):
        problems.append(
            f"marker center {marker_center} is not the marker color "
            f"(a vertical flip puts the marker below center)"
        )
    if any(max(c) > 40 for c in corners):
        problems.append(f"corners are not clear color: {corners}")
    problems += expect_bbox("sprite", sprite_pixels, args.sprite, expected_sprite)
    problems += expect_bbox("marker", marker_pixels, args.marker, expected_marker)

    if problems:
        print("BEVY_SPIKE_PPM_FAILED")
        for problem in problems:
            print(f"  {problem}")
        sys.exit(1)
    print(f"BEVY_SPIKE_PPM_PASSED size={width}x{height} sprite={len(sprite_pixels)} "
          f"marker={len(marker_pixels)} sprite_center_x={sprite_center_x} center={center} corner={corners[0]}")


if __name__ == "__main__":
    main()

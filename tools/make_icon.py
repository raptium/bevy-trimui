#!/usr/bin/env python3
"""Brick Pro Bevy demo icon: orange sprite and blue orientation marker."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw


def build():
    plate = Image.new("RGBA", (40, 40))
    drawing = ImageDraw.Draw(plate)
    drawing.rounded_rectangle((0, 0, 39, 39), radius=6, fill=(13, 13, 25, 255))
    drawing.rectangle((11, 16, 29, 34), fill=(255, 127, 0, 255))
    drawing.rectangle((17, 5, 23, 11), fill=(26, 102, 255, 255))
    icon = Image.new("RGBA", (60, 60))
    icon.alpha_composite(plate, (10, 10))
    return icon.resize((300, 300), Image.Resampling.NEAREST)


def main(out_path):
    target = Path(out_path)
    target.parent.mkdir(parents=True, exist_ok=True)
    icon = build()
    icon.save(target, "PNG")
    assert icon.getchannel("A").getbbox() == (50, 50, 250, 250)
    assert set(icon.getchannel("A").get_flattened_data()) == {0, 255}
    print(f"wrote {target} (demo sprite and marker, 5x nearest scaling)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output")
    main(parser.parse_args().output)

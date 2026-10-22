#!/usr/bin/env python3
"""Decode a raw evdev byte stream captured from the Brick.

Usage:  python3 tools/device/decode_evdev.py <raw-file>

struct input_event on 64-bit Linux is 24 bytes:
    0..15  struct timeval { __kernel_long_t tv_sec, tv_usec }
    16..17 __u16 type, 18..19 __u16 code, 20..23 __s32 value
"""

import argparse
import struct
from collections import defaultdict

BTN = {
    0x130: "BTN_SOUTH", 0x131: "BTN_EAST", 0x133: "BTN_NORTH",
    0x134: "BTN_WEST", 0x136: "BTN_TL(LB)", 0x137: "BTN_TR(RB)",
    0x138: "BTN_TL2", 0x139: "BTN_TR2",
    0x13a: "BTN_SELECT", 0x13b: "BTN_START", 0x13c: "BTN_MODE(MENU)",
    0x13d: "BTN_THUMBL", 0x13e: "BTN_THUMBR",
    0x220: "BTN_DPAD_UP", 0x221: "BTN_DPAD_DOWN",
    0x222: "BTN_DPAD_LEFT", 0x223: "BTN_DPAD_RIGHT",
}

ABS = {
    0x00: "ABS_X", 0x01: "ABS_Y", 0x02: "ABS_Z", 0x03: "ABS_RX",
    0x04: "ABS_RY", 0x05: "ABS_RZ", 0x06: "ABS_THROTTLE",
    0x07: "ABS_RUDDER", 0x08: "ABS_WHEEL", 0x09: "ABS_GAS",
    0x0a: "ABS_BRAKE", 0x10: "ABS_HAT0X", 0x11: "ABS_HAT0Y",
}


def main(path):
    with open(path, "rb") as source:
        data = source.read()
    if len(data) % 24:
        raise ValueError("incomplete evdev event: expected 24-byte records")
    n = len(data) // 24
    if n == 0:
        print("no complete events")
        return

    print(f"{n} events from {path}\n")

    axis_ranges = defaultdict(lambda: [None, None])
    buttons = defaultdict(set)

    for i in range(n):
        sec, usec, typ, code, val = struct.unpack_from("<qqHHi", data, i * 24)
        if typ == 1:  # EV_KEY
            name = BTN.get(code, f"code 0x{code:03x}")
            buttons[name].add(val)
        elif typ == 3:  # EV_ABS
            name = ABS.get(code, f"ABS 0x{code:02x}")
            r = axis_ranges[name]
            r[0] = val if r[0] is None else min(r[0], val)
            r[1] = val if r[1] is None else max(r[1], val)

    print("buttons seen (code -> name -> values):")
    for name in sorted(buttons, key=lambda s: s):
        print(f"  {name:22s} values={sorted(buttons[name])}")

    print("\naxes seen (name -> min..max, centre):")
    for name in sorted(axis_ranges):
        lo, hi = axis_ranges[name]
        print(f"  {name:22s} {lo} .. {hi}   centre={(lo + hi) // 2}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_file")
    main(parser.parse_args().raw_file)

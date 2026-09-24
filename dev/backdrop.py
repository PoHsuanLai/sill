#!/usr/bin/env python3
"""The acceptance scripts' test wallpaper: soft colour bands, no photo, so the bar's and the
dock's blur and tint have something behind them. A 2176 x 1360 PNG (the nested output's size
on this machine), drawn by upscaling a small grid of band colours with bicubic filtering, so
it is smooth and the same on every run.

Usage: dev/backdrop.py OUT.png
"""
import math
import sys

from PIL import Image

WIDTH, HEIGHT = 2176, 1360
# Band colours, top-left to bottom-right: dusk blue, teal, sand, coral, plum.
STOPS = [(46, 74, 140), (38, 138, 150), (226, 196, 140), (222, 112, 96), (104, 64, 128)]


def band(t: float) -> tuple:
    """The colour a fraction `t` (0..1) along the diagonal, eased between the stops."""
    t = min(max(t, 0.0), 1.0) * (len(STOPS) - 1)
    i = min(int(t), len(STOPS) - 2)
    f = t - i
    f = f * f * (3 - 2 * f)
    a, b = STOPS[i], STOPS[i + 1]
    return tuple(round(a[k] + (b[k] - a[k]) * f) for k in range(3))


def main() -> None:
    out = sys.argv[1]
    cols, rows = 68, 43
    small = Image.new("RGB", (cols, rows))
    for y in range(rows):
        for x in range(cols):
            u, v = x / (cols - 1), y / (rows - 1)
            wave = 0.06 * math.sin(6.0 * u + 2.0 * v)
            small.putpixel((x, y), band(0.62 * u + 0.38 * v + wave))
    small.resize((WIDTH, HEIGHT), Image.BICUBIC).save(out)


if __name__ == "__main__":
    main()

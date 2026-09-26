#!/usr/bin/env python3
r"""Prints a live CSV stream for trying `termplt --follow`: a noisy signal and its moving average.

    python3 scripts/live_data.py | termplt -f -x time -y raw,smoothed --xlim 0,15 --ylim -2,2 \
        --marker none --line-thickness 1 --title "Live signal" --legend-loc lower-left

Writes a `time,raw,smoothed` header, then one row every 1/RATE seconds for SECONDS seconds
(forever with 0). The noise is seeded, so every run draws the same plot.
"""

import math
import random

import live_stream


def rows(args, dt):
    rng = random.Random(1)
    smoothed = None
    for t in live_stream.times(dt):
        raw = math.sin(t) + 0.4 * math.sin(3.1 * t) + rng.gauss(0, 0.15)
        # an exponential moving average
        smoothed = raw if smoothed is None else smoothed + 0.15 * (raw - smoothed)
        yield raw, smoothed


if __name__ == "__main__":
    live_stream.run(__doc__, ["raw", "smoothed"], rows)

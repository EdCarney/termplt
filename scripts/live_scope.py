#!/usr/bin/env python3
r"""Prints a two-channel "oscilloscope" CSV stream for `termplt --follow`: a sine and a square wave.

    python3 scripts/live_scope.py | termplt -f -x time -y sine,square --window 200 --ylim -1.5,1.5 \
        --marker none --line-thickness 1 --title "Oscilloscope" --legend-loc upper-left

Writes a `time,sine,square` header, then one row every 1/RATE seconds for SECONDS seconds
(forever with 0). The square wave is the sum of its first HARMONICS odd harmonics,
(4/pi) sum sin(2 pi n f t) / n, so it rings at its edges (the Gibbs phenomenon) like a
band-limited signal on a real scope; more harmonics give sharper edges. At the defaults a
200-point window shows two periods, and the fixed --ylim keeps the vertical scale steady.
"""

import math

import live_stream


def add_args(parser):
    parser.add_argument(
        "--freq", type=float, default=0.5, help="frequency of both waves in Hz (default: 0.5)"
    )
    parser.add_argument(
        "--harmonics",
        type=int,
        default=5,
        help="odd harmonics summed for the square wave (default: 5)",
    )


def rows(args, dt):
    w = 2 * math.pi * args.freq
    orders = range(1, 2 * args.harmonics, 2)
    for t in live_stream.times(dt):
        sine = math.sin(w * t)
        square = 4 / math.pi * sum(math.sin(n * w * t) / n for n in orders)
        yield sine, square


if __name__ == "__main__":
    live_stream.run(__doc__, ["sine", "square"], rows, add_args, rate=50, seconds=30)

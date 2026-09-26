#!/usr/bin/env python3
r"""Prints a live CSV stream for trying `termplt --follow`: a noisy signal and its moving average.

    python3 scripts/live_data.py | termplt -f -x time -y raw,smoothed --window 200 \
        --marker none --line-thickness 1 --title "Live signal"

Writes a `time,raw,smoothed` header, then one row every 1/RATE seconds for SECONDS seconds
(forever with 0). The noise is seeded, so every run draws the same plot.
"""

import argparse
import math
import os
import random
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--rate", type=float, default=20, help="rows per second (default: 20)")
    parser.add_argument(
        "--seconds", type=float, default=15, help="how long to run; 0 runs until stopped (default: 15)"
    )
    args = parser.parse_args()

    rng = random.Random(1)
    period = 1 / args.rate
    rows = math.inf if args.seconds == 0 else round(args.seconds * args.rate)
    smoothed = None
    start = time.monotonic()
    print("time,raw,smoothed", flush=True)
    k = 0
    while k < rows:
        t = k * period
        raw = math.sin(t) + 0.4 * math.sin(3.1 * t) + rng.gauss(0, 0.15)
        smoothed = raw if smoothed is None else smoothed + 0.15 * (raw - smoothed)
        print(f"{t:.2f},{raw:.4f},{smoothed:.4f}", flush=True)
        k += 1
        # paced against the start, so the rate doesn't drift
        time.sleep(max(0.0, start + k * period - time.monotonic()))


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        # Ctrl-C: stop quietly, without a traceback over the live plot
        sys.exit(130)
    except BrokenPipeError:
        # termplt exited; point stdout at /dev/null so the exit flush doesn't fail again
        # (https://docs.python.org/3/library/signal.html#note-on-sigpipe)
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        sys.exit(1)

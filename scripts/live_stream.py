"""Shared by the live_*.py demo scripts: prints a CSV stream at a steady rate for `termplt --follow`.

A script supplies a header and a generator of rows; `run` adds the `--rate`/`--seconds` options,
prints `time,<columns>` rows paced against the start, and exits quietly on Ctrl-C or when
termplt closes the pipe, so no traceback lands on the live plot.
"""

import argparse
import itertools
import math
import os
import sys
import time


def run(doc, columns, rows, add_args=None, rate=20, seconds=15):
    """Streams `rows(args, dt)`, a generator of value tuples for t = 0, dt, 2 dt, ..., as CSV.

    `doc` is the script's docstring (its first line is the --help description), `columns` the
    names after `time`, `add_args(parser)` adds the script's own options, and `rate`/`seconds`
    are the defaults for rows per second and run time.
    """
    try:
        _stream(doc, columns, rows, add_args, rate, seconds)
    except KeyboardInterrupt:
        sys.exit(130)
    except BrokenPipeError:
        # termplt exited; point stdout at /dev/null so the exit flush doesn't fail again
        # (https://docs.python.org/3/library/signal.html#note-on-sigpipe)
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        sys.exit(1)


def _stream(doc, columns, rows, add_args, rate, seconds):
    parser = argparse.ArgumentParser(description=doc.strip().splitlines()[0])
    parser.add_argument(
        "--rate", type=float, default=rate, help=f"rows per second (default: {rate:g})"
    )
    parser.add_argument(
        "--seconds",
        type=float,
        default=seconds,
        help=f"how long to run; 0 runs until stopped (default: {seconds:g})",
    )
    if add_args:
        add_args(parser)
    args = parser.parse_args()
    if args.rate <= 0:
        parser.error("--rate must be positive")

    period = 1 / args.rate
    count = math.inf if args.seconds == 0 else round(args.seconds * args.rate)
    start = time.monotonic()
    print(",".join(["time", *columns]), flush=True)
    for k, values in enumerate(rows(args, period)):
        if k >= count:
            break
        print(",".join([f"{k * period:.3f}", *(f"{v:.4f}" for v in values)]), flush=True)
        # paced against the start, so the rate doesn't drift
        time.sleep(max(0.0, start + (k + 1) * period - time.monotonic()))


def times(dt):
    """t = 0, dt, 2 dt, ...: computed from the index, so rounding doesn't accumulate."""
    return (k * dt for k in itertools.count())

#!/usr/bin/env python3
r"""Prints a live CSV stream of a mass-spring-damper for `termplt --follow`: x, v and a.

    python3 scripts/live_oscillator.py | termplt -f -x time -y position,velocity,acceleration \
        --window 500 --marker none --line-thickness 1 --title "Mass-spring-damper" --legend-loc lower-left

Writes a `time,position,velocity,acceleration` header, then one row every 1/RATE seconds for
SECONDS seconds (forever with 0). The system is x'' + 2 zeta omega x' + omega^2 x = F(t), from
rest, with a force that steps between +omega^2 and -omega^2 every HOLD seconds, so each step
moves the rest position between +1 and -1 and the mass overshoots and rings down to it. The
acceleration jumps at each step (a = F - 2 zeta omega v - omega^2 x), the velocity peaks
where the position crosses zero, and the three differ in phase by about 90 degrees each. At
omega = 1 rad/s they share a scale (v ~ omega x, a ~ omega^2 x); a 500-point window shows 25 s.
"""

import math

import live_stream

# RK4 steps per row
SUBSTEPS = 10


def add_args(parser):
    parser.add_argument(
        "--omega", type=float, default=1.0, help="natural frequency in rad/s (default: 1)"
    )
    parser.add_argument(
        "--zeta", type=float, default=0.2, help="damping ratio; 1 is critical (default: 0.2)"
    )
    parser.add_argument(
        "--hold", type=float, default=12, help="seconds between force reversals (default: 12)"
    )


def rows(args, dt):
    w, zeta = args.omega, args.zeta
    hold = max(1, round(args.hold / dt))  # in rows, so every reversal lands on a row
    x, v = 0.0, 0.0
    h = dt / SUBSTEPS

    def accel(x, v, force):
        return force - 2 * zeta * w * v - w * w * x

    k = 0
    while True:
        force = w * w * (1 if (k // hold) % 2 == 0 else -1)
        yield x, v, accel(x, v, force)
        # the force is constant until the next row
        for _ in range(SUBSTEPS):
            k1x, k1v = v, accel(x, v, force)
            k2x, k2v = v + h / 2 * k1v, accel(x + h / 2 * k1x, v + h / 2 * k1v, force)
            k3x, k3v = v + h / 2 * k2v, accel(x + h / 2 * k2x, v + h / 2 * k2v, force)
            k4x, k4v = v + h * k3v, accel(x + h * k3x, v + h * k3v, force)
            x += h / 6 * (k1x + 2 * k2x + 2 * k3x + k4x)
            v += h / 6 * (k1v + 2 * k2v + 2 * k3v + k4v)
        k += 1


if __name__ == "__main__":
    live_stream.run(
        __doc__, ["position", "velocity", "acceleration"], rows, add_args, seconds=60
    )

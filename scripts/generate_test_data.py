#!/usr/bin/env python3
"""Generate test data files for validating termplt behavior."""

import math
import os
import random

OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "test_data")


def write_csv(filename, header, rows):
    path = os.path.join(OUTPUT_DIR, filename)
    with open(path, "w") as f:
        if header:
            f.write(header + "\n")
        for x, y in rows:
            f.write(f"{x},{y}\n")
    print(f"  {filename:30s} ({len(rows)} points)")


def random_scatter(n=80, seed=42):
    """Uniform random points -- good for testing scatter plots (--line_style None)."""
    rng = random.Random(seed)
    return [(rng.uniform(-10, 10), rng.uniform(-10, 10)) for _ in range(n)]


def random_clusters(n_per_cluster=30, seed=7):
    """Three Gaussian clusters -- scatter plot with visible grouping."""
    rng = random.Random(seed)
    centers = [(2, 3), (-4, -2), (5, -5)]
    points = []
    for cx, cy in centers:
        for _ in range(n_per_cluster):
            points.append((rng.gauss(cx, 0.8), rng.gauss(cy, 0.8)))
    return points


def sine(n=100):
    """One full period of sin(x) over [0, 2pi]."""
    return [(x, math.sin(x)) for x in frange(0, 2 * math.pi, n)]


def cosine(n=100):
    """One full period of cos(x) over [0, 2pi]."""
    return [(x, math.cos(x)) for x in frange(0, 2 * math.pi, n)]


def parabola(n=100):
    """y = x^2 over [-5, 5]."""
    return [(x, x * x) for x in frange(-5, 5, n)]


def cubic(n=100):
    """y = x^3 over [-3, 3]."""
    return [(x, x ** 3) for x in frange(-3, 3, n)]


def exponential(n=80):
    """y = e^x over [-2, 3]."""
    return [(x, math.exp(x)) for x in frange(-2, 3, n)]


def logarithm(n=80):
    """y = ln(x) over [0.1, 10]."""
    return [(x, math.log(x)) for x in frange(0.1, 10, n)]


def damped_sine(n=150):
    """y = e^(-x/3) * sin(3x) over [0, 12] -- damped oscillation."""
    return [
        (x, math.exp(-x / 3) * math.sin(3 * x))
        for x in frange(0, 12, n)
    ]


def circle(n=120):
    """Parametric circle (x=cos(t), y=sin(t)) -- tests equal-aspect rendering."""
    return [
        (math.cos(t), math.sin(t))
        for t in frange(0, 2 * math.pi, n)
    ]


def lissajous(n=200, a=3, b=2):
    """Lissajous figure x=sin(at), y=sin(bt) -- tests dense curves."""
    return [
        (math.sin(a * t), math.sin(b * t))
        for t in frange(0, 2 * math.pi, n)
    ]


def noisy_linear(n=60, seed=99):
    """y = 2x + 1 + noise -- tests scatter with a visible trend."""
    rng = random.Random(seed)
    xs = list(frange(-5, 5, n))
    return [(x, 2 * x + 1 + rng.gauss(0, 1.5)) for x in xs]


def step_function(n=100):
    """Piecewise step function -- tests sharp transitions."""
    points = []
    for x in frange(-5, 5, n):
        if x < -2:
            y = -1
        elif x < 1:
            y = 0
        elif x < 3:
            y = 2
        else:
            y = 1
        points.append((x, y))
    return points


# ── helpers ──────────────────────────────────────────────────────────────────


def frange(start, stop, n):
    """Yield n evenly spaced floats from start to stop (inclusive)."""
    step = (stop - start) / (n - 1)
    return [start + i * step for i in range(n)]


# ── main ─────────────────────────────────────────────────────────────────────


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    print(f"Writing test data to {os.path.abspath(OUTPUT_DIR)}/\n")

    # CSV with header
    write_csv("sine.csv", "x,y", sine())
    write_csv("cosine.csv", "x,y", cosine())
    write_csv("parabola.csv", "x,y", parabola())
    write_csv("cubic.csv", "x,y", cubic())
    write_csv("exponential.csv", "x,y", exponential())
    write_csv("logarithm.csv", "x,y", logarithm())
    write_csv("damped_sine.csv", "x,y", damped_sine())
    write_csv("circle.csv", "x,y", circle())
    write_csv("lissajous.csv", "x,y", lissajous())
    write_csv("step_function.csv", "x,y", step_function())

    # No header (raw numeric data)
    write_csv("random_scatter.csv", None, random_scatter())
    write_csv("random_clusters.csv", None, random_clusters())
    write_csv("noisy_linear.csv", None, noisy_linear())

    print(f"\nDone! Example commands:\n")
    print(f"  # Single series with lines")
    print(f"  cargo run -- --data_file test_data/sine.csv")
    print()
    print(f"  # Scatter plot (no lines)")
    print(f"  cargo run -- --data_file test_data/random_scatter.csv --line_style None")
    print()
    print(f"  # Multi-series overlay")
    print(f"  cargo run -- --data_file test_data/sine.csv --data_file test_data/cosine.csv")
    print()
    print(f"  # Styled scatter")
    print(f"  cargo run -- --data_file test_data/random_clusters.csv \\")
    print(f"    --line_style None --marker_style FilledCircle --marker_color Cyan --marker_size 3")
    print()
    print(f"  # Parametric curves")
    print(f"  cargo run -- --data_file test_data/circle.csv --marker_style None --line_thickness 1")
    print()
    print(f"  # Damped oscillation")
    print(f"  cargo run -- --data_file test_data/damped_sine.csv --marker_style None --line_color Lime")


if __name__ == "__main__":
    main()

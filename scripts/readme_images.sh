#!/usr/bin/env bash
# Regenerates the README images in docs/images/ with the current CLI. Each command matches the
# one shown above its image in README.md. Run after a visual change and review the PNGs.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 scripts/generate_test_data.py > /dev/null
cargo build --release --quiet
termplt=target/release/termplt
out=docs/images
mkdir -p "$out"

"$termplt" --data "(1,1),(2,4),(3,9),(4,16)" -o "$out/inline-points.png"
"$termplt" test_data/random_clusters.csv --line none -o "$out/scatter.png"
"$termplt" test_data/sine.csv test_data/cosine.csv -o "$out/multiple-series.png"
"$termplt" test_data/lissajous.csv --marker hollow-circle --marker-size 4 --color cyan \
    --line-thickness 1 -o "$out/custom-style.png"
"$termplt" test_data/noisy_linear.csv --marker none --color lime --line-thickness 1 \
    --xlim 0,10 -o "$out/line-limits.png"

"$termplt" test_data/sine.csv test_data/cosine.csv --title "Sine and cosine" \
    --xlabel "angle (rad)" --ylabel "value" -o "$out/titles.png"

echo "Wrote $(ls "$out" | wc -l | tr -d ' ') images to $out/"

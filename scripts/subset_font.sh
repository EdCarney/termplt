#!/usr/bin/env bash
# Rebuilds the embedded font (assets/fonts/go-regular-subset.ttf) and the digits-only test font
# (tests/fixtures/digits-only.ttf) from the pinned Go Regular release. Needs python3 and network
# access; a pinned fonttools is installed into a temporary virtualenv. The output is
# deterministic, so an unchanged run leaves `git status` clean.
set -euo pipefail
cd "$(dirname "$0")/.."

# golang/image commit "font/gofont: upgrade to version 2.010"
commit=41969df76e82aeec85fa3821b1e24955ea993001
sha256=197d9f3703b4c00af609178876a8d73e396f64fe438b2c871778566632374be3
url="https://raw.githubusercontent.com/golang/image/$commit/font/gofont/ttfs/Go-Regular.ttf"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -sSfL -o "$work/Go-Regular.ttf" "$url"
if command -v sha256sum >/dev/null; then
    echo "$sha256  $work/Go-Regular.ttf" | sha256sum -c - >/dev/null
else
    echo "$sha256  $work/Go-Regular.ttf" | shasum -a 256 -c - >/dev/null
fi

python3 -m venv "$work/venv"
"$work/venv/bin/pip" install --quiet fonttools==4.66.0

# --name-IDs='*' keeps the copyright and license records; --notdef-outline keeps the box drawn
# for characters the font lacks; ab_glyph ignores hinting, so it is dropped
subset() {
    "$work/venv/bin/pyftsubset" "$work/Go-Regular.ttf" --no-hinting --name-IDs='*' \
        --notdef-outline "$@"
}
subset --unicodes-file=assets/fonts/charset.txt --output-file=assets/fonts/go-regular-subset.ttf
mkdir -p tests/fixtures
subset --unicodes=U+0030-0039 --output-file=tests/fixtures/digits-only.ttf
ls -l assets/fonts/go-regular-subset.ttf tests/fixtures/digits-only.ttf

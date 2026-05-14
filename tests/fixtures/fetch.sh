#!/usr/bin/env bash
# Fetches the official LZEXE 0.91 binary distribution from bellard.org into
# lzexe/tests/fixtures/ for use by the integration test suite.
#
# The binaries are distributed by Fabrice Bellard from his own site alongside
# the MIT-licensed source. Run this once before `cargo test -p lzexe`.
#
# Usage: ./lzexe/tests/fixtures/fetch.sh

set -euo pipefail

FIXTURES_DIR="$(cd "$(dirname "$0")" && pwd)"
ZIP_URL="https://bellard.org/lzexe/lzexe91.zip"
TMP_ZIP="$(mktemp /tmp/lzexe91_XXXXXX.zip)"

echo "Downloading $ZIP_URL ..."
curl -fsSL "$ZIP_URL" -o "$TMP_ZIP"

for name in LZEXE.EXE UPACKEXE.EXE; do
    unzip -p "$TMP_ZIP" "$name" > "$FIXTURES_DIR/$name"
    echo "  extracted $name ($(wc -c < "$FIXTURES_DIR/$name") bytes)"
done

rm -f "$TMP_ZIP"
echo "Done. Run: cargo test -p lzexe"

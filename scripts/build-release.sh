#!/usr/bin/env bash
# Build release with Brotli compression for nginx
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "Building release..."
trunk build --release

echo "Compressing with Brotli (parallel, max compression)..."
# Use parallel compression with all cores, quality 11 (max)
find dist -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.html" \) | \
    xargs -P $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) -I {} brotli -f -k -q 11 {}

echo ""
echo "=== Build complete ==="
echo ""
echo "Original vs Brotli:"
for f in dist/*.wasm dist/*.js; do
    if [ -f "$f" ] && [ -f "$f.br" ]; then
        orig=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
        comp=$(stat -f%z "$f.br" 2>/dev/null || stat -c%s "$f.br")
        ratio=$((100 - comp * 100 / orig))
        printf "  %-45s %8d -> %8d (%d%% smaller)\n" "$(basename $f)" "$orig" "$comp" "$ratio"
    fi
done

echo ""
echo "Upload dist/ to acadlisp.de"
echo "nginx will serve .br files automatically for browsers that support it"

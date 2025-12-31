#!/usr/bin/env bash
# Build release with Brotli compression for nginx
# Usage:
#   ./build-release.sh          # Just build
#   ./build-release.sh local    # Build and start local server on port 8053
#   ./build-release.sh deploy   # Build and deploy to acadlisp.de

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

MODE="${1:-build}"

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

case "$MODE" in
    local)
        echo "=== Starting local server on port 8053 ==="
        echo "Access at: http://127.0.0.1:8053/"
        echo "Press Ctrl+C to stop"
        echo ""
        trunk serve --release --port 8053
        ;;
    deploy)
        echo "=== Deploying to acadlisp.de ==="
        echo "Syncing dist/ to trahe.eu:/var/www/acadlisp.de/html ..."
        rsync -avz --progress dist/ trahe.eu:/var/www/acadlisp.de/html/
        echo ""
        echo "Deployed! Visit https://acadlisp.de/"
        ;;
    build|*)
        echo "Build only. Use './build-release.sh local' to start server or './build-release.sh deploy' to deploy."
        ;;
esac

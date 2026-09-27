#!/usr/bin/env bash
# Native XLISP 3.3 benchmark driver, using the same wrapper (xlisp_init /
# xlisp_eval) as the Emscripten build on acadlisp.de, so native and wasm
# numbers measure the same code path.
#   XLISP_DIR=../../../xlisp ./build.sh      (default: clone of github.com/holg/xlisp next to acadlisp)
#   ./xlisp-bench ../programs/fib.xl [runs]
# NOTE: the upstream Makefile compiles without optimisation; we build the
# library with -O2 to match the -O2 Emscripten build.
set -euo pipefail
cd "$(dirname "$0")"
XLISP_DIR="${XLISP_DIR:-../../../xlisp}"
if [ ! -d "$XLISP_DIR" ]; then
  echo "clone https://github.com/holg/xlisp to $XLISP_DIR or set XLISP_DIR" >&2; exit 1
fi
( cd "$XLISP_DIR" && make clean >/dev/null 2>&1 || true; make CFLAGS="-O2 -DUNIX -Iinclude" library )
sed -e 's/#include <emscripten.h>//' -e 's/EMSCRIPTEN_KEEPALIVE//' "$XLISP_DIR/wasm_wrapper.c" > native_wrapper.c
cc -O2 -DUNIX -I"$XLISP_DIR" -I"$XLISP_DIR/include" native_bench.c -L"$XLISP_DIR/lib" -lxlisp -lm -o xlisp-bench
echo "ok: ./xlisp-bench ../programs/<name>.xl [runs]"

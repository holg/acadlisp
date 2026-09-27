# kisp vs acadlisp: an extensive comparison

Date: 2026-09-27.
kisp is Kyrill Sokoll's Lisp interpreter in Rust (github.com/holg/kisp, fork of chebureki/kisp, last upstream commit April 2023, ~1950 lines).
acadlisp is the AutoLISP interpreter and mini CAD engine behind acadlisp.de (v0.2.0, ~10500 lines including HP41, KiCad, DXF and the WASM API).
All numbers below come from `kisp-compare/`, a harness that builds both crates into one binary and runs the same programs in both dialects (see "Reproducing").

## 1. Summary

The two projects have different goals and it shows in every layer.
kisp is a small, clean, functional language with immutable values, persistent cons lists, a proper error type with stack traces, and tail call optimisation.
acadlisp is a domain interpreter: it re-implements 1988 AutoLISP semantics (uppercase symbols, dynamic scoping, nil/T, integer division) and bolts a drawing state, CSV batch processing, DXF/SVG/KiCad export and a wasm_bindgen API on top.

On call-heavy code kisp is 4x to 14x slower than acadlisp as checked out today.
The dominant cost is not the language design but one implementation detail: kisp creates every function frame as a child of the *calling* scope (dynamic scoping), so every symbol lookup walks a chain as long as the recursion depth.
A 7-line experimental patch (frames are children of the *defining* scope) makes fib(22) 6.3x faster, fixes closures, and leaves all kisp tests green.
On list-heavy code kisp is already 3x faster than acadlisp, because acadlisp's `cons` copies a `Vec` while kisp shares cons cells behind `Rc`.

For Odysseus-rs the answer is clear: kisp would not perform better, and it could not do the job at all, because it has no strings, no drawing primitives and no host-state extension API, and because the LLM-facing value of acadlisp is that models already know AutoLISP.
What Odysseus and acadlisp should borrow from kisp is the robustness: a call depth guard (added to acadlisp in this session), a structured error type instead of strings pushed to `output`, and a persistent list representation.

## 2. Benchmarks

Environment: Cowork Linux VM on Holger's Mac (aarch64, 4 cores), Rust 1.98.1, `--release` with LTO, fresh interpreter per run, 20 runs after 3 warm-ups, median in ms.
Both engines include environment creation in "eval", which is what a user pays per script on acadlisp.de or per `/api/cad` request in Odysseus.

| program | what it does | kisp (as is) | kisp (lexical patch) | acadlisp | ratio as is | ratio patched |
|---|---|---:|---:|---:|---:|---:|
| fib | fib(22), naive recursion | 391 | 62 | 28 | 14.0x | 2.2x |
| loop_sum | sum 1..50000, TCO vs WHILE | 71 | 63 | 10 | 7.0x | 6.2x |
| list_ops | cons 1500, map, fold | 3.1 | 2.9 | 9.1 | 0.34x | 0.32x |
| call_overhead | 20000 trivial calls | 26 | 22 | 5.7 | 4.7x | 3.9x |
| cad_grid | 900 circles + 900 lines to SVG | 4.2 (numeric core only) | 2.7 | 1.3 (full render) | n/a | n/a |

### 2.1 After the patches (branches `kisp/perf/lexical-scope` and `acadlisp/perf/kisp-inspired`)

Same machine, same harness, both crates on their patched branches:

| program | kisp before | kisp after | acadlisp before | acadlisp after | ratio after |
|---|---:|---:|---:|---:|---:|
| fib | 391 | 62 | 28 | 12 | 5.2x |
| loop_sum | 71 | 64 | 10 | 10 | 6.3x |
| list_ops | 3.1 | 2.9 | 9.1 | 8.9 | 0.32x |
| call_overhead | 26 | 23 | 5.7 | 4.7 | 4.9x |
| cad_grid | 4.2 | 2.7 | 1.3 | 1.4 | n/a |

kisp: fib 6.3x faster (lexical scoping), closures work, tail calls inside `[block]` bodies work, parse 2 to 3x faster (byte-offset lexer), deep recursion still yields `StackOverflow` (call counter, limit 420).
acadlisp: fib 2.3x faster (no `UserFunction` clone per call), deep recursion yields an error at 500 nested calls instead of aborting the process, `mapcar`/`apply` fixed.
The remaining 5x on call-heavy code is kisp's per-call `Rc<Scope>` + `HashMap` frame and the `Rc<Expression>` wrapping of builtin arguments; the remaining 3x on lists is acadlisp's `Vec` copying in `cons`/`cdr`.
One trade-off in kisp: the stack trace is built from the scope chain, which is now short, so traces list only the lexically enclosing functions, not the dynamic call stack.

### 2.2 Native vs WebAssembly, with XLISP 3.3 as the third engine

acadlisp.de already ships XLISP 3.3 (David Betz, github.com/holg/xlisp fork, Emscripten `-O2`) for its benchmark slide, so it is the natural yardstick: a 1980s C bytecode compiler and VM against two Rust tree-walking interpreters.
The XLISP programs are in `programs/*.xl` (Scheme dialect: `define`, named `let`, `map`, own `fold`).
WASM numbers come from Node v22 (V8) on the same VM: kisp and acadlisp through `kisp-compare/wasm` (wasm-bindgen, `opt-level = 3`, no wasm-opt), XLISP through the `xlisp.js`/`xlisp.wasm` shipped in the acadlisp repo.
Native XLISP goes through the same `xlisp_init`/`xlisp_eval` wrapper as the wasm build, compiled with `-O2` (the upstream Makefile uses no optimisation at all).
All engines on the patched branches, 20 runs, medians in ms:

| program | kisp native | kisp wasm | acadlisp native | acadlisp wasm | xlisp native | xlisp wasm |
|---|---:|---:|---:|---:|---:|---:|
| fib | 62 | 85 | 12 | 17 | 3.7 | 14 |
| loop_sum | 64 | 95 | 10 | 14 | 4.0 | 16 |
| list_ops | 2.9 | 4.1 | 8.9 | 19 | 0.32 | 1.4 |
| call_overhead | 23 | 33 | 4.7 | 6.8 | 2.0 | 7.7 |
| cad_grid | 2.7 | 3.9 | 1.4 (full render) | 2.2 (full render) | 0.26 | 1.1 |

What the matrix says:

Native XLISP wins everything by 3x to 30x: it compiles to bytecode once and runs a tight C VM, both Rust engines re-walk the AST and allocate per call.
The gap is much smaller in WebAssembly, where XLISP loses 3.5x to 4.5x against its native build (function pointer emulation, `longjmp` support and the Emscripten runtime cost), while the Rust engines lose only 1.3x to 1.5x. In the browser acadlisp and XLISP are within 20 percent of each other on call-heavy code.
acadlisp's `Vec` lists hurt twice as much in wasm (list_ops 8.9 to 19 ms): the allocator behind `Vec` copies is where wasm is weakest.
kisp keeps its list advantage in wasm (4.1 ms vs 19 ms) and is the only engine whose wasm penalty is uniform.
The Odysseus and acadlisp.de render path (cad_grid, full SVG export) is 2.2 ms in wasm; still not the bottleneck of either pipeline.

Reproduce the whole matrix with `kisp-compare/run_all.sh` after `wasm/build.sh` (needs `wasm32-unknown-unknown` and `wasm-bindgen-cli` 0.2.129) and `xlisp/build.sh` (needs a clone of github.com/holg/xlisp next to acadlisp or `XLISP_DIR`).

Parse times are 1 to 20 microseconds for both and irrelevant.
acadlisp parses two to three times faster because its lexer works on byte offsets, while kisp's lexer calls `chars().nth(i)` for every character, which is O(n) per character and therefore O(n^2) per file; this only matters for large sources.

Observations per program:

fib is pure call overhead plus symbol lookup.
kisp allocates an `Rc<Scope>` with a `RefCell<HashMap>` per call, wraps every argument of a builtin (`if`, `+`, `<`, `-` are all builtins) in an `Rc<ReferenceValue::Expression(clone)>`, and walks the scope chain for every lookup.
acadlisp keeps a `Vec<HashMap>` scope stack and matches builtins with a string `match`, which is a simple but effective dispatch.
acadlisp also clones the whole `UserFunction` (params + body) on every call (`self.functions.get(name).cloned()`), which is its own low-hanging fruit.

loop_sum shows that kisp's tail call optimisation works (50000 iterations inside a 420-frame limit) but costs a `scope.clear()`, a `HashMap` re-insert and a `TailCall` allocation per iteration.
acadlisp's `WHILE` with `SETQ` is a plain loop.

list_ops is where kisp's design wins.
`cons` is O(1) and lists share structure; acadlisp's `cons` allocates a new `Vec` and copies, so building a list of n with `cons` is O(n^2).
`map`, `filter`, `fold`, `reduce`, `zip`, `enumerate`, `flatten` exist natively in kisp with lambdas; acadlisp has `mapcar` and `apply` with named functions only.

call_overhead exposed a kisp bug: a tail call inside a `[...]` block is not detected, because `eval_block` opens a fresh scope with `origin: None`, and `is_tail_call` compares the origin of the *current* scope with the called function.
`(fn loop [n] (if (>= 0 n) 0 [(id n) (loop (- n 1))]))` therefore hits `StackOverflow` at depth 420.
The benchmark was rewritten to keep the call in tail position of the `if`.

cad_grid is the Odysseus-rs render path (`ody-services::cad::render`: `WasmEngine::new`, `eval`, `get_entities_svg`).
1800 entities and a 132 KB SVG in 1.3 ms means the interpreter is not the bottleneck of that pipeline; the LLM round trip is.
kisp can only express the arithmetic half (build the 900 points as a list) and needs twice the time for that.

## 3. Language and semantics

| aspect | kisp | acadlisp |
|---|---|---|
| dialect | own design, `fn`, `let`, `lambda`, `[block]` | AutoLISP R9/R10, `defun`, `setq`, `progn`, `cond`, `while`, `repeat`, `foreach` |
| symbols | case sensitive | uppercased at lex time (AutoLISP style) |
| atoms | i32, f64, `unit`, `true`, list, callable, quoted expression | nil, T, i32, f64, string, symbol, list, quote |
| strings | none | yes, with `strcat`, `substr`, `strcase`, `atoi`, `rtos`, ... |
| falsity | `unit` is false, everything else true | nil and `()` are false |
| integer overflow | `checked_*`, promotes to f64 | wraps silently (`(+ 2147483647 1)` gives -2147483648, which is also what AutoLISP did) |
| division | `(/ 7 2)` = 3.5, always float | `(/ 7 2)` = 3, AutoLISP integer division |
| scoping | main: dynamic in practice (frames are children of the caller); `in_scope` is stored but unused, so closures fail: `(fn mk [n] (lambda [y] (+ n y)))` gives `UnknownSymbol("n")`. Branch: lexical, closures work | dynamic, as in AutoLISP (`locals` stack searched innermost first, then globals; `*name*` forces global) |
| tail calls | yes, trampolined `TailCall` value; on main not inside `[block]` (fixed on the branch) | no |
| recursion limit | 420 (scope chain depth on main, call counter on the branch), returns `StackOverflow` error with a trace | none on main; deep recursion aborted the process (natively at about 20000 frames, in WASM much earlier). On the branch `MAX_CALL_DEPTH = 500` yields an AutoLISP error |
| errors | `Result<_, ErrorContext>` with `EvalError` enum and a stack trace of function names; parser errors carry a cursor | none in the type system; errors are strings pushed to `output`, evaluation continues with nil; parser silently drops unexpected `)` |
| source positions | every AST node has a `Cursor` (line, column, offset) | none |
| quoting and macros | `quote`, `eval`, builtins receive unevaluated expressions so special forms are ordinary builtins | `QUOTE`, `EVAL`, `APPLY`; special forms hard-coded in `eval_list` |
| higher order | `map filter fold reduce zip enumerate flatten` with lambdas | `mapcar` and `apply` with named functions; `LAMBDA` evaluates to nil |
| lists | persistent singly linked `Rc<Con>` cells, `car cdr cons nth list` | `Vec<Expr>` with `car cdr cadr caddr cons list append reverse length nth last member assoc subst` |
| host integration | none (no foreign function API, `print` goes to stdout) | `register_fn` with `ForeignContext` (output, globals, drawing state, user data), `command`, `getvar`/`setvar`, file I/O, CSV batch, DXF/SVG/KiCad export |
| WASM | library compiles to wasm32 once `linefeed` is optional (done in this session) | cdylib with wasm_bindgen, shipped on acadlisp.de |
| tests | 8 integration tests | 36 unit tests in lib, 13 in the CLI, plus 3 new bridge tests |
| REPL | `linefeed` based, multi line with `\` | `acadlisp-cli -i` |

## 4. Code quality notes

kisp reads like a language implementation textbook: lexer, parser, AST with positions, value model split into copyable and `Rc` referenced values, stdlib in modules, and a `func("name", callback)` registration table.
Adding a builtin is one function and one line.
The weak spots are the ones that come with a two-month project: `chars().nth()` in the lexer, deep AST clones when a builtin is called, `stdlib/types.rs` registers `float` with the `int` callback and `is-unit?` four times, and the REPL greeting string should be removed before anybody else sees it.

acadlisp has grown by use.
`interpreter.rs` is a 2200-line `match` on uppercase strings; the interpreter clones function bodies per call, `mapcar` and `apply` re-evaluated their already evaluated arguments (so `(mapcar 'car '((1 2) (3 4)))` returned `(nil nil)`; fixed in this session, it now returns `(1 3)`), and nothing is `Result`.
For its purpose, deterministic rendering of small programs written by humans or LLMs, that is fine, and the numbers show it.

## 5. What each side could take from the other

For kisp, applied on branch `perf/lexical-scope` of the holg fork (worth sending upstream as a PR):

1. Function and lambda frames are children of `in_scope` (lexical scoping); the self tail call is detected against the caller's nearest enclosing function origin.
   A function stored in the scope it captures forms an `Rc` cycle; a REPL or script run does not care, a long-running embedder would want `Weak` or an arena.
2. Tail calls inside `[block]` bodies: the block's tail context is now propagated to its last element, and block scopes are transparent for the origin check.
3. Recursion limit: a thread-local call counter (`MAX_CALL_DEPTH = 420`) with an RAII guard replaces the scope chain depth, which no longer grows.
4. AST lists and blocks are `Rc<Vec<PosExpression>>`, so wrapping builtin arguments is an `Rc` bump.
5. Lexer works on byte offsets (`chars().nth(i)` was O(n) per character); multi-byte characters in comments are handled.
6. `float` was registered with the `int` callback, `is-unit?` three times, `is_list?` gained the `is-list?` spelling; the REPL greeting is replaced.
7. `linefeed` optional behind the default `repl` feature, so `--no-default-features` builds on wasm32.
8. Seven new tests (closures, lexical scoping, deep recursion, block tail calls, float, multi-byte comment).

For acadlisp, applied on branch `perf/kisp-inspired`:

1. Call depth guard: `MAX_CALL_DEPTH = 500` (each Lisp call costs several Rust frames; 500 fits the 1 MB WASM stack), reported as an AutoLISP error.
2. `mapcar` / `apply` quote their already evaluated arguments.
3. `functions` holds `Arc<UserFunction>`: no clone of params and body per call, fib 28 ms to 12 ms.
4. Two regression tests.

Still open for acadlisp: represent lists as shared cons cells or `Rc<Vec>` so `cons` and `cdr` stop copying, and structured errors (`Expr::Error` or `Result` with a source position) so acadlisp.de and Odysseus can show *where* an LLM-generated program went wrong instead of an empty drawing.

## 6. kisp and the web tools

The `kisp` cargo feature in acadlisp (added in this session) embeds kisp in the engine:

* `WasmEngine::eval_kisp(code)` evaluates a kisp program in a fresh environment and returns the printed value or `Error: ...`.
* `(KISP "<source>")` is registered as an AutoLISP foreign function, so AutoLISP code can call into kisp and use the result: `(setq squares (kisp "(map (lambda [x] (* x x)) (list 1 2 3))"))` gives `(1 4 9)`.
  Integers, floats, lists, `unit` (nil) and `true` (T) convert; callables come back as strings.
* `engine_info()` lists `"kisp"` in `features` when the build has it.
* `index.html`: the playground runs a program whose first line is `;kisp` through `eval_kisp`, and `data-cargo-features="kisp"` on the trunk link enables the feature in the acadlisp.de build (+146 KB wasm before wasm-opt).

bimifc.de is a separate repository that was not available in this session.
If it embeds the acadlisp crate, enabling the `kisp` feature there gives it the same `(KISP ...)` function; if it does not, the cleanest hook is `acadlisp::kisp_bridge::eval_kisp_to_expr` behind a small wasm_bindgen function, since kisp itself has no wasm_bindgen surface.

Odysseus-rs (`crates/ody-services/src/cad.rs`) depends on `acadlisp = "0.2"` from crates.io, so it picks the bridge up once 0.3 is published with the feature; `acadlisp = { version = "0.3", features = ["kisp"] }` would make `(KISP ...)` available to LLM-written programs.
There is no reason to do that today: nothing in the CAD prompt benefits from a second dialect, and the LLM prompt is tuned to AutoLISP.

## 7. Reproducing

```
cd acadlisp/kisp-compare
cargo run --release              # all benchmarks, 20 runs each
cargo run --release -- 50 fib    # 50 runs, programs matching "fib"
cargo run --release -- run a.kisp b.lsp   # evaluate single files (semantic probes)
```

The first experiment (lexical scoping only, without the call counter and block fix) is kept as `kisp-compare/patches/kisp-lexical-scope-experiment.diff`; the complete change is the `perf/lexical-scope` branch of kisp.

Files touched in this session:

* kisp branch `perf/lexical-scope`: `Cargo.toml`, `src/ast.rs`, `src/parser.rs`, `src/lexer.rs`, `src/interpreter.rs`, `src/scope.rs`, `src/stdlib/types.rs`, `src/bin/repl.rs`, `tests/`.
* acadlisp branch `perf/kisp-inspired`: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/kisp_bridge.rs`, `src/interpreter.rs`, `index.html`, `kisp-compare/` (native harness, `wasm/` Node runner, `xlisp/` C driver, `run_all.sh`), this document.

// Benchmark runner for the WebAssembly builds: kisp + acadlisp (pkg/, built by
// build.sh) and XLISP 3.3 (../../xlisp.js, the Emscripten build shipped on
// acadlisp.de). Same programs and the same protocol as the native harness:
// fresh interpreter per run, 3 warm-ups, N timed runs, min / median / mean in ms.
//
//   node bench.mjs [runs] [filter]
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const rs = require('./pkg/kisp_compare_wasm.js');

const runs = parseInt(process.argv[2] || '20', 10);
const filter = process.argv[3] || '';

const PROGRAMS = [
  ['fib', 'fib(22), naive recursion'],
  ['loop_sum', 'sum 1..50000'],
  ['list_ops', 'cons 1500 / map / fold'],
  ['call_overhead', '20000 trivial calls'],
  ['cad_grid', 'Odysseus render path (kisp/xlisp: numeric core only)'],
];

function bench(f) {
  for (let i = 0; i < 3; i++) f();
  const t = [];
  for (let i = 0; i < runs; i++) {
    const a = performance.now();
    f();
    t.push(performance.now() - a);
  }
  t.sort((x, y) => x - y);
  return { min: t[0], median: t[Math.floor(runs / 2)], mean: t.reduce((a, b) => a + b, 0) / runs };
}

// ---- XLISP (Emscripten module, pthread build: needs worker_threads in Node)
let xl = null;
try {
  const createXLisp = require(join(here, '..', '..', 'xlisp.js'));
  xl = await createXLisp();
  if (!xl._xlisp_init()) xl = null;
} catch (e) {
  console.error('XLISP wasm not available:', e.message);
}
function xlispEval(src) {
  // xlisp_eval evaluates one form: wrap the file in (begin ...), like native_bench.c
  const bytes = new TextEncoder().encode('(begin ' + src + ' )\0');
  const ptr = xl._malloc(bytes.length);
  xl.HEAPU8.set(bytes, ptr);
  const out = xl.UTF8ToString(xl._xlisp_eval(ptr));
  xl._free(ptr);
  return out;
}

const read = (name, ext) => readFileSync(join(here, '..', 'programs', `${name}.${ext}`), 'utf8');
const f3 = (x) => x.toFixed(3).padStart(9);

console.log(`wasm benchmarks in Node ${process.version}: ${runs} runs per cell (after 3 warm-ups), ms\n`);
let md = '| program | engine | wasm median | wasm min | result |\n|---|---|---:|---:|---|\n';
for (const [name, desc] of PROGRAMS) {
  if (filter && !name.includes(filter)) continue;
  const ksrc = read(name, 'kisp'), asrc = read(name, 'lsp');
  const rows = [
    ['kisp', bench(() => rs.eval_kisp(ksrc)), rs.eval_kisp(ksrc)],
    ['acadlisp', bench(() => rs.eval_acad(asrc)), rs.eval_acad(asrc)],
  ];
  if (xl) {
    const xsrc = read(name, 'xl');
    rows.push(['xlisp', bench(() => xlispEval(xsrc)), xlispEval(xsrc)]);
  }
  console.log(`${name}: ${desc}`);
  for (const [engine, st, res] of rows) {
    console.log(`  ${engine.padEnd(9)} ${f3(st.min)} ${f3(st.median)} ${f3(st.mean)}   ${res.slice(0, 60)}`);
    md += `| ${name} | ${engine} | ${st.median.toFixed(3)} | ${st.min.toFixed(3)} | \`${res.slice(0, 40)}\` |\n`;
  }
}
console.log('\n--- markdown ---\n' + md);

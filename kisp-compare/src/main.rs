//! kisp-compare: side-by-side micro benchmarks for kisp (Kyrill's Lisp) and acadlisp.
//!
//! Each benchmark exists twice in `programs/` (`.kisp` and `.lsp`), written in the
//! idiomatic form of each dialect but computing the same value.
//! For every program we measure, with a fresh interpreter per run:
//!   * parse  - lexing + parsing only
//!   * eval   - parse + fresh environment + evaluation (what a user pays per script)
//! Timings are min / median / mean over N runs after a warm-up.
//!
//! Usage:  cargo run --release -- [runs] [filter]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

/// Counting allocator for the `mem` subcommand: live bytes, peak live bytes,
/// total bytes and number of allocations since the last reset.
struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(l.size(), Relaxed) + l.size();
        PEAK.fetch_max(live, Relaxed);
        TOTAL.fetch_add(l.size(), Relaxed);
        COUNT.fetch_add(1, Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        System.dealloc(p, l)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn mem_reset() {
    PEAK.store(LIVE.load(Relaxed), Relaxed);
    TOTAL.store(0, Relaxed);
    COUNT.store(0, Relaxed);
}


const PROGRAMS: &[(&str, &str)] = &[
    ("fib", "fib(22), naive recursion"),
    ("loop_sum", "sum 1..50000 (TCO vs WHILE)"),
    ("list_ops", "cons 1500 / map / fold"),
    ("call_overhead", "20000 trivial calls"),
    ("cad_grid", "Odysseus render path: 900 circles + 900 lines -> SVG (kisp: numeric core only)"),
];

fn read(name: &str, ext: &str) -> String {
    let path = format!("{}/programs/{}.{}", env!("CARGO_MANIFEST_DIR"), name, ext);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// ---------------------------------------------------------------- kisp

fn kisp_parse(src: &str) -> kisp::ast::PosExpression {
    let lexer = kisp::lexer::Lexer::from_text(src);
    let mut iter = lexer.into_iter();
    kisp::parser::parse(&mut iter).expect("kisp parse error")
}

fn kisp_eval(src: &str) -> String {
    let ast = kisp_parse(src);
    let (res, _env) = kisp::interpreter::eval(&ast, None);
    match res {
        Ok((v, _)) => v.to_string(),
        Err(e) => format!("ERR {e:?}"),
    }
}

// ---------------------------------------------------------------- acadlisp

fn acad_parse(src: &str) -> Vec<acadlisp::Expr> {
    acadlisp::parser::Parser::from_input(src).parse_all()
}

/// The path Odysseus-rs (ody-services::cad::render) takes: WasmEngine eval + SVG export.
fn acad_render_svg(src: &str) -> String {
    let mut eng = acadlisp::WasmEngine::new();
    let log = eng.eval(src);
    let svg = eng.get_entities_svg();
    format!("{} entities, {} bytes svg, last={}", eng.entity_count(), svg.len(), log.rsplit(',').next().unwrap_or(""))
}

fn acad_eval(src: &str) -> String {
    if src.contains("(command ") {
        return acad_render_svg(src);
    }
    let exprs = acad_parse(src);
    let mut it = acadlisp::Interpreter::new();
    let mut last = acadlisp::Expr::Nil;
    for e in &exprs {
        last = it.eval_top(e);
    }
    last.to_string()
}

// ---------------------------------------------------------------- harness

struct Stats {
    min: Duration,
    median: Duration,
    mean: Duration,
}

fn bench<F: FnMut()>(runs: usize, mut f: F) -> Stats {
    for _ in 0..3 {
        f();
    }
    let mut samples: Vec<Duration> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .collect();
    samples.sort();
    let mean = samples.iter().sum::<Duration>() / runs as u32;
    Stats { min: samples[0], median: samples[runs / 2], mean }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    // `kisp-compare run <file.kisp|file.lsp>` evaluates one file and prints the result
    // (handy for semantic probes, the kisp REPL needs a terminal).
    if let Some(first) = args.next() {
        if first == "mem" {
            // one evaluation per program and engine, allocation statistics
            println!("| program | engine | peak live KB | allocated KB | allocations |\n|---|---|---:|---:|---:|");
            for (name, _) in PROGRAMS {
                let ksrc = read(name, "kisp");
                let asrc = read(name, "lsp");
                mem_reset();
                std::hint::black_box(kisp_eval(&ksrc));
                let k = (PEAK.load(Relaxed) / 1024, TOTAL.load(Relaxed) / 1024, COUNT.load(Relaxed));
                mem_reset();
                std::hint::black_box(acad_eval(&asrc));
                let a = (PEAK.load(Relaxed) / 1024, TOTAL.load(Relaxed) / 1024, COUNT.load(Relaxed));
                println!("| {name} | kisp | {} | {} | {} |\n| {name} | acadlisp | {} | {} | {} |", k.0, k.1, k.2, a.0, a.1, a.2);
            }
            return;
        }
        if first == "run" {
            for path in args {
                let src = std::fs::read_to_string(&path).expect("read file");
                let out = if path.ends_with(".lsp") { acad_eval(&src) } else { kisp_eval(&src) };
                let out: String = out.chars().take(300).collect();
                println!("{path} -> {out}");
            }
            return;
        }
        // not a subcommand: treat as run count
        run_bench(first.parse().unwrap_or(20), args.next());
        return;
    }
    run_bench(20, None);
}

fn run_bench(runs: usize, filter: Option<String>) {
    println!("kisp-compare: {runs} runs per cell (after 3 warm-ups), times in ms\n");
    println!(
        "{:<14} {:<9} {:<6} {:>9} {:>9} {:>9}   result",
        "program", "engine", "phase", "min", "median", "mean"
    );
    println!("{}", "-".repeat(78));

    let mut md = String::from("| program | engine | parse (median) | eval (median) | eval (min) | result |\n|---|---|---:|---:|---:|---|\n");

    for (name, desc) in PROGRAMS {
        if let Some(f) = &filter {
            if !name.contains(f.as_str()) {
                continue;
            }
        }
        let ksrc = read(name, "kisp");
        let asrc = read(name, "lsp");

        let kres = kisp_eval(&ksrc);
        let ares = acad_eval(&asrc);

        let kp = bench(runs, || {
            std::hint::black_box(kisp_parse(&ksrc));
        });
        let ke = bench(runs, || {
            std::hint::black_box(kisp_eval(&ksrc));
        });
        let ap = bench(runs, || {
            std::hint::black_box(acad_parse(&asrc));
        });
        let ae = bench(runs, || {
            std::hint::black_box(acad_eval(&asrc));
        });

        println!("{name}: {desc}");
        for (engine, phase, st, res) in [
            ("kisp", "parse", &kp, ""),
            ("kisp", "eval", &ke, kres.as_str()),
            ("acadlisp", "parse", &ap, ""),
            ("acadlisp", "eval", &ae, ares.as_str()),
        ] {
            println!(
                "{:<14} {:<9} {:<6} {:>9.3} {:>9.3} {:>9.3}   {}",
                "", engine, phase, ms(st.min), ms(st.median), ms(st.mean), res
            );
        }
        let ratio = ke.median.as_secs_f64() / ae.median.as_secs_f64();
        println!(
            "{:<14} eval median ratio kisp/acadlisp = {:.2}x\n",
            "", ratio
        );
        md.push_str(&format!(
            "| {name} | kisp | {:.3} | {:.3} | {:.3} | `{}` |\n| {name} | acadlisp | {:.3} | {:.3} | {:.3} | `{}` |\n",
            ms(kp.median), ms(ke.median), ms(ke.min), kres,
            ms(ap.median), ms(ae.median), ms(ae.min), ares
        ));
    }

    println!("\n--- markdown ---\n{md}");
}

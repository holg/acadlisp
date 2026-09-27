//! Thin wasm-bindgen exports so `bench.mjs` can time kisp and acadlisp inside
//! a WebAssembly runtime (Node) with exactly the same programs as the native run.
use wasm_bindgen::prelude::*;

/// Parse + fresh environment + evaluate a kisp program; returns the printed value.
#[wasm_bindgen]
pub fn eval_kisp(src: &str) -> String {
    let lexer = kisp::lexer::Lexer::from_text(src);
    let mut iter = lexer.into_iter();
    let ast = match kisp::parser::parse(&mut iter) {
        Ok(a) => a,
        Err(e) => return format!("ERR {e:?}"),
    };
    match kisp::interpreter::eval(&ast, None).0 {
        Ok((v, _)) => v.to_string(),
        Err(e) => format!("ERR {e:?}"),
    }
}

/// Same as the native harness: plain eval, or the WasmEngine render path
/// (eval + SVG export) when the program draws with (command ...).
#[wasm_bindgen]
pub fn eval_acad(src: &str) -> String {
    if src.contains("(command ") {
        let mut eng = acadlisp::WasmEngine::new();
        let log = eng.eval(src);
        let svg = eng.get_entities_svg();
        return format!("{} entities, {} bytes svg, last={}", eng.entity_count(), svg.len(), log.rsplit(',').next().unwrap_or(""));
    }
    let exprs = acadlisp::parser::Parser::from_input(src).parse_all();
    let mut it = acadlisp::Interpreter::new();
    let mut last = acadlisp::Expr::Nil;
    for e in &exprs {
        last = it.eval(e);
    }
    last.to_string()
}

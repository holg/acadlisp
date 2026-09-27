//! Bridge between acadlisp and kisp (Kyrill's Lisp, <https://github.com/holg/kisp>).
//!
//! Enabled with the `kisp` cargo feature. It offers two entry points:
//!
//! * [`eval_kisp`] / [`eval_kisp_to_expr`]: evaluate a kisp program and get the
//!   result back as a kisp value or converted to an acadlisp [`Expr`].
//! * [`register`]: registers a `(KISP "<source>")` foreign function in an
//!   acadlisp [`Interpreter`], so AutoLISP code running on acadlisp.de (and any
//!   other embedder of the engine, e.g. bimifc.de) can call into kisp and use the
//!   result like any other AutoLISP value:
//!
//! ```lisp
//! (setq squares (kisp "(map (lambda [x] (* x x)) (list 1 2 3))"))
//! ; => (1 4 9)
//! ```
//!
//! kisp values map to AutoLISP values as follows: integers and floats to
//! `Integer`/`Real`, `unit` to `nil`, `true` to `T`, lists to lists (recursively),
//! everything else (functions, quoted expressions) to its printed form as a string.
//! Every call evaluates in a fresh kisp environment; kisp scopes are `Rc` based and
//! therefore cannot be captured by the `Send + Sync` foreign-function closure.

use crate::interpreter::Interpreter;
use crate::parser::Expr;
use kisp::value::numeric::Numeric;
use kisp::value::{EvalValue, ReferenceValue};

/// Evaluate kisp source in a fresh environment.
pub fn eval_kisp(src: &str) -> Result<EvalValue, String> {
    let lexer = kisp::lexer::Lexer::from_text(src);
    let mut tokens = lexer.into_iter();
    let ast = kisp::parser::parse(&mut tokens).map_err(|e| format!("kisp parse error: {e:?}"))?;
    let (result, _env) = kisp::interpreter::eval(&ast, None);
    result
        .map(|(value, _ctx)| value)
        .map_err(|e| format!("kisp eval error: {e:?}"))
}

/// Convert a kisp value into an acadlisp expression.
pub fn to_expr(value: &EvalValue) -> Expr {
    match value {
        EvalValue::Numeric(Numeric::Integer(i)) => Expr::Integer(*i),
        EvalValue::Numeric(Numeric::Floating(f)) => Expr::Real(*f),
        EvalValue::Unit => Expr::Nil,
        EvalValue::True => Expr::T,
        EvalValue::Reference(r) => match r.as_ref() {
            ReferenceValue::List(list) => {
                let items: Vec<Expr> = list.iterator().map(|v| to_expr(&v)).collect();
                if items.is_empty() {
                    Expr::Nil
                } else {
                    Expr::List(items)
                }
            }
            other => Expr::String(other.to_string()),
        },
    }
}

/// Evaluate kisp source and convert the result; errors become a string starting with `"Error: "`.
pub fn eval_kisp_to_expr(src: &str) -> Expr {
    match eval_kisp(src) {
        Ok(v) => to_expr(&v),
        Err(e) => Expr::String(format!("Error: {e}")),
    }
}

/// Register the `(KISP "<source>")` foreign function.
pub fn register(interp: &mut Interpreter) {
    interp.register_fn("KISP", |_ctx, args| match args.first() {
        Some(Expr::String(src)) => eval_kisp_to_expr(src),
        _ => Expr::String("Error: KISP expects a source string".to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_lists_round_trip() {
        assert_eq!(eval_kisp_to_expr("(+ 1 2)"), Expr::Integer(3));
        assert_eq!(eval_kisp_to_expr("(/ 7 2)"), Expr::Real(3.5));
        assert_eq!(eval_kisp_to_expr("()"), Expr::Nil);
        assert_eq!(
            eval_kisp_to_expr("(map (lambda [x] (* x x)) (list 1 2 3))"),
            Expr::List(vec![Expr::Integer(1), Expr::Integer(4), Expr::Integer(9)])
        );
    }

    #[test]
    fn callable_from_autolisp() {
        let mut interp = Interpreter::new();
        register(&mut interp);
        let out = interp.run("(setq r (kisp \"(fold 0 (lambda [a b] (+ a b)) (list 1 2 3 4))\")) (+ r 1)");
        assert_eq!(out.last(), Some(&Expr::Integer(11)));
    }

    #[test]
    fn errors_are_strings() {
        assert!(matches!(eval_kisp_to_expr("(undefined 1)"), Expr::String(s) if s.starts_with("Error:")));
        assert!(matches!(eval_kisp_to_expr("(+ 1"), Expr::String(s) if s.starts_with("Error:")));
    }
}

//! Compilation of `defun` bodies into a resolved tree ("closure compilation").
//!
//! A user function's body is compiled once, on its first call, from the generic
//! `Expr` tree into `Node`s in which everything that can be decided ahead of
//! time already is:
//!
//! * a parameter or declared local of the function is a frame slot index
//!   (`Node::Local`), read and written without a symbol scan;
//! * a constant (number, string, `nil`, `T`, a quoted form) is a ready value;
//! * a builtin call holds the `Builtin` enum and the compiled argument nodes;
//! * a call to a user or foreign function holds the interned symbol and the
//!   compiled arguments; the function itself is looked up at call time, since
//!   `defun` may redefine it later;
//! * the control forms (`setq if cond while repeat progn and or foreach`) are
//!   nodes with compiled children.
//!
//! What AutoLISP does not allow us to resolve stays dynamic: a free variable is
//! looked up by symbol through the caller frames at run time (`Node::Var`), and
//! forms the compiler does not handle (`defun`, `mapcar`, `lambda`, and anything
//! unusual) fall back to the generic evaluator on the original expression
//! (`Node::Generic`), so semantics never change, only the speed.
//!
//! Top-level forms outside a `defun` are still evaluated by the generic walker.

use crate::interpreter::Interpreter;
use crate::parser::Expr;
use crate::symbol::{Builtin, Sym};
use smallvec::SmallVec;

/// Where a `setq` or `foreach` writes.
#[derive(Debug)]
pub enum Target {
    Local(usize),
    Var(Sym),
}

#[derive(Debug)]
pub enum Node {
    Const(Expr),
    Local(usize),
    Var(Sym),
    Setq(Vec<(Target, Node)>),
    If(Box<Node>, Box<Node>, Option<Box<Node>>),
    Cond(Vec<(Node, Vec<Node>)>),
    While(Box<Node>, Vec<Node>),
    Repeat(Box<Node>, Vec<Node>),
    Progn(Vec<Node>),
    And(Vec<Node>),
    Or(Vec<Node>),
    Foreach(Target, Box<Node>, Vec<Node>),
    Builtin(Builtin, Vec<Node>),
    Call(Sym, Vec<Node>),
    Generic(Expr),
}

struct Compiler<'a> {
    params: &'a [Sym],
}

impl<'a> Compiler<'a> {
    fn slot(&self, sym: Sym) -> Option<usize> {
        // last occurrence wins, matching the frame scan order in get_var
        self.params.iter().rposition(|p| *p == sym)
    }

    fn target(&self, sym: Sym) -> Target {
        match self.slot(sym) {
            Some(i) => Target::Local(i),
            None => Target::Var(sym),
        }
    }

    fn seq(&self, forms: &Expr) -> Vec<Node> {
        forms.list_iter().map(|e| self.expr(e)).collect()
    }

    fn expr(&self, e: &Expr) -> Node {
        match e {
            Expr::Nil | Expr::T | Expr::Integer(_) | Expr::Real(_) | Expr::String(_) => {
                Node::Const(e.clone())
            }
            Expr::Quote(inner) => Node::Const((**inner).clone()),
            Expr::Symbol(s) => match self.slot(*s) {
                Some(i) => Node::Local(i),
                None => Node::Var(*s),
            },
            Expr::List(list) => {
                let head = match list.car() {
                    Expr::Symbol(s) => *s,
                    _ => return Node::Const(Expr::Nil), // not callable in AutoCAD 9/10
                };
                let args = list.cdr();
                match head.builtin() {
                    Some(Builtin::Quote) => Node::Const(args.nth(0).cloned().unwrap_or(Expr::Nil)),
                    Some(Builtin::Setq) => {
                        let mut pairs = Vec::new();
                        let mut rest = args;
                        while let (Some(sym), Some(value)) = (rest.nth(0), rest.nth(1)) {
                            if let Expr::Symbol(s) = sym {
                                pairs.push((self.target(*s), self.expr(value)));
                            }
                            rest = rest.tail(2);
                        }
                        Node::Setq(pairs)
                    }
                    Some(Builtin::If) => match args.nth(0) {
                        None => Node::Const(Expr::Nil),
                        Some(test) => Node::If(
                            Box::new(self.expr(test)),
                            Box::new(args.nth(1).map(|e| self.expr(e)).unwrap_or(Node::Const(Expr::Nil))),
                            args.nth(2).map(|e| Box::new(self.expr(e))),
                        ),
                    },
                    Some(Builtin::Cond) => Node::Cond(
                        args.list_iter()
                            .filter_map(|clause| match clause {
                                Expr::List(items) => Some((self.expr(items.car()), self.seq(items.cdr()))),
                                _ => None,
                            })
                            .collect(),
                    ),
                    Some(Builtin::While) => match args.nth(0) {
                        None => Node::Const(Expr::Nil),
                        Some(test) => Node::While(Box::new(self.expr(test)), self.seq(args.tail(1))),
                    },
                    Some(Builtin::Repeat) => match args.nth(0) {
                        None => Node::Const(Expr::Nil),
                        Some(count) => Node::Repeat(Box::new(self.expr(count)), self.seq(args.tail(1))),
                    },
                    Some(Builtin::Progn) => Node::Progn(self.seq(args)),
                    Some(Builtin::And) => Node::And(self.seq(args)),
                    Some(Builtin::Or) => Node::Or(self.seq(args)),
                    Some(Builtin::Foreach) => match (args.nth(0), args.nth(1)) {
                        (Some(Expr::Symbol(var)), Some(list_expr)) => Node::Foreach(
                            self.target(*var),
                            Box::new(self.expr(list_expr)),
                            self.seq(args.tail(2)),
                        ),
                        _ => Node::Const(Expr::Nil),
                    },
                    // defun / lambda / mapcar keep the generic evaluator
                    Some(Builtin::Defun) | Some(Builtin::Lambda) | Some(Builtin::Mapcar) => {
                        Node::Generic(e.clone())
                    }
                    Some(b) => Node::Builtin(b, self.seq(args)),
                    None => Node::Call(head, self.seq(args)),
                }
            }
        }
    }
}

/// Compile a function body for a frame laid out as `params` (parameters first,
/// then the locals after `/`, exactly as `call_user_function` builds it).
pub fn compile_body(params: &[Sym], body: &[Expr]) -> Vec<Node> {
    let c = Compiler { params };
    body.iter().map(|e| c.expr(e)).collect()
}

impl Node {
    #[inline]
    fn run_seq(nodes: &[Node], it: &mut Interpreter) -> Expr {
        let mut result = Expr::Nil;
        for n in nodes {
            result = n.run(it);
        }
        result
    }

    #[inline]
    fn assign(target: &Target, value: Expr, it: &mut Interpreter) {
        match target {
            Target::Local(i) => {
                if let Some(frame) = it.locals.last_mut() {
                    frame[*i].1 = value;
                }
            }
            Target::Var(s) => it.set_var(*s, value),
        }
    }

    pub fn run(&self, it: &mut Interpreter) -> Expr {
        match self {
            Node::Const(v) => v.clone(),
            Node::Local(i) => it.locals.last().map(|f| f[*i].1.clone()).unwrap_or(Expr::Nil),
            Node::Var(s) => it.get_var(*s),
            Node::Setq(pairs) => {
                let mut result = Expr::Nil;
                for (target, value) in pairs {
                    let v = value.run(it);
                    Node::assign(target, v.clone(), it);
                    result = v;
                }
                result
            }
            Node::If(test, then, els) => {
                if test.run(it).is_truthy() {
                    then.run(it)
                } else {
                    match els {
                        Some(e) => e.run(it),
                        None => Expr::Nil,
                    }
                }
            }
            Node::Cond(clauses) => {
                for (test, body) in clauses {
                    let t = test.run(it);
                    if t.is_truthy() {
                        return if body.is_empty() { t } else { Node::run_seq(body, it) };
                    }
                }
                Expr::Nil
            }
            Node::While(test, body) => {
                let mut result = Expr::Nil;
                while test.run(it).is_truthy() {
                    result = Node::run_seq(body, it);
                }
                result
            }
            Node::Repeat(count, body) => {
                let n = count.run(it).as_integer().unwrap_or(0);
                let mut result = Expr::Nil;
                for _ in 0..n {
                    result = Node::run_seq(body, it);
                }
                result
            }
            Node::Progn(body) => Node::run_seq(body, it),
            Node::And(items) => {
                for n in items {
                    if !n.run(it).is_truthy() {
                        return Expr::Nil;
                    }
                }
                Expr::T
            }
            Node::Or(items) => {
                for n in items {
                    let v = n.run(it);
                    if v.is_truthy() {
                        return v;
                    }
                }
                Expr::Nil
            }
            Node::Foreach(target, list, body) => {
                let list = list.run(it);
                let mut result = Expr::Nil;
                if let Expr::List(items) = list {
                    for item in items.iter() {
                        Node::assign(target, item.clone(), it);
                        result = Node::run_seq(body, it);
                    }
                }
                result
            }
            Node::Builtin(b, args) => {
                let evaled: SmallVec<[Expr; 8]> = args.iter().map(|a| a.run(it)).collect();
                if it.is_shadowed(*b) {
                    it.call_named(Sym::from_builtin(*b), &evaled)
                } else {
                    it.call_builtin(*b, &evaled)
                }
            }
            Node::Call(sym, args) => {
                let evaled: SmallVec<[Expr; 8]> = args.iter().map(|a| a.run(it)).collect();
                it.call_named(*sym, &evaled)
            }
            Node::Generic(e) => it.eval(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::parser::Expr;

    fn run(src: &str) -> String {
        let mut it = Interpreter::new();
        it.run(src).last().map(|e| e.to_string()).unwrap_or_default()
    }

    #[test]
    fn dynamic_scoping_through_compiled_code() {
        // inner has no binding for x: setq reaches outer's parameter (slot)
        assert_eq!(run("(defun inner () (setq x 9)) (defun outer (x) (inner) x) (outer 1)"), "9");
        // and reads see the caller's binding
        assert_eq!(run("(defun show () x) (defun outer (x) (show)) (outer 4)"), "4");
    }

    #[test]
    fn locals_after_slash_are_slots_bound_to_nil() {
        assert_eq!(run("(setq t1 100) (defun f (a / t1) (setq t1 (+ a 1)) t1) (list (f 1) t1)"), "(2 100)");
    }

    #[test]
    fn control_forms_compile() {
        assert_eq!(
            run("(defun f (n / acc i) (setq acc 0 i 0) (while (< i n) (setq i (1+ i)) (cond ((= (rem i 2) 0) (setq acc (+ acc i))) (t nil))) (repeat 2 (setq acc (1+ acc))) (foreach k '(1 2) (setq acc (+ acc k))) (if (and acc (or nil acc)) acc -1)) (f 10)"),
            "35"
        );
    }

    #[test]
    fn redefinition_after_first_call_is_picked_up() {
        let mut it = Interpreter::new();
        it.run("(defun g () 1) (defun f () (g))");
        assert_eq!(it.run("(f)")[0], Expr::Integer(1));
        it.run("(defun g () 2)");
        assert_eq!(it.run("(f)")[0], Expr::Integer(2));
    }

    #[test]
    fn integer_fast_paths_wrap_like_autolisp() {
        assert_eq!(run("(defun f (a b) (+ a b)) (f 2147483647 1)"), "-2147483648");
        assert_eq!(run("(defun f (a b) (< a b)) (f 1 2)"), "T");
        assert_eq!(run("(defun f (a b) (- a b)) (f 1 2.5)"), "-1.5");
    }
}

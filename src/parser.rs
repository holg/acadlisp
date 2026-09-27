// AutoLISP Parser - S-expression parser for AutoCAD 9/10 dialect

use crate::lexer::{Lexer, Token};

use std::sync::Arc;

/// One cons cell. `cdr` is `Expr::List` for the rest of a proper list, `Expr::Nil`
/// at the end, or any other atom for a dotted pair `(a . b)`.
#[derive(Debug, Clone)]
pub struct Cons {
    pub car: Expr,
    pub cdr: Expr,
}

/// A non-empty AutoLISP list: a shared, immutable cons chain. The empty list is
/// `Expr::Nil`, as in AutoLISP. Cloning a list or taking its `cdr` is O(1) and
/// shares structure, `cons` allocates one cell; the whole chain is never copied.
/// `Arc` rather than `Rc` keeps `Expr` (and every embedding engine) `Send`.
#[derive(Debug, Clone)]
pub struct List(Arc<Cons>);

impl List {
    /// Build one cell in front of `cdr`.
    pub fn cons(car: Expr, cdr: Expr) -> List {
        List(Arc::new(Cons { car, cdr }))
    }

    /// Build a proper list; `None` for an empty vector (that is `Expr::Nil`).
    pub fn from_vec(items: Vec<Expr>) -> Option<List> {
        let mut tail = Expr::Nil;
        for item in items.into_iter().rev() {
            tail = Expr::List(List::cons(item, tail));
        }
        match tail {
            Expr::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn car(&self) -> &Expr {
        &self.0.car
    }

    pub fn cdr(&self) -> &Expr {
        &self.0.cdr
    }

    /// Iterate the proper part of the list (stops at a dotted tail).
    pub fn iter(&self) -> ListIter<'_> {
        ListIter { next: Some(self) }
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn first(&self) -> Option<&Expr> {
        Some(&self.0.car)
    }

    pub fn get(&self, n: usize) -> Option<&Expr> {
        self.iter().nth(n)
    }

    pub fn last(&self) -> Option<&Expr> {
        self.iter().last()
    }

    pub fn to_vec(&self) -> Vec<Expr> {
        self.iter().cloned().collect()
    }

    /// True if the last cdr is an atom other than nil: `(a . b)`.
    pub fn is_dotted(&self) -> bool {
        let mut cur = self;
        loop {
            match cur.cdr() {
                Expr::List(l) => cur = l,
                Expr::Nil => return false,
                _ => return true,
            }
        }
    }
}

impl PartialEq for List {
    fn eq(&self, other: &List) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        let (mut a, mut b) = (self, other);
        loop {
            if a.car() != b.car() {
                return false;
            }
            match (a.cdr(), b.cdr()) {
                (Expr::List(x), Expr::List(y)) => {
                    a = x;
                    b = y;
                }
                (x, y) => return x == y,
            }
        }
    }
}

impl std::ops::Index<usize> for List {
    type Output = Expr;
    fn index(&self, n: usize) -> &Expr {
        self.get(n).expect("list index out of range")
    }
}

pub struct ListIter<'a> {
    next: Option<&'a List>,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a Expr;
    fn next(&mut self) -> Option<&'a Expr> {
        let cell = self.next?;
        self.next = match cell.cdr() {
            Expr::List(l) => Some(l),
            _ => None,
        };
        Some(cell.car())
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = &'a Expr;
    type IntoIter = ListIter<'a>;
    fn into_iter(self) -> ListIter<'a> {
        self.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Nil,
    T,
    Integer(i32),
    Real(f64),
    String(String),
    Symbol(String),
    List(List),
    Quote(Box<Expr>),
}

impl Expr {
    /// A proper list from a vector; an empty vector is `Nil`.
    pub fn list(items: Vec<Expr>) -> Expr {
        match List::from_vec(items) {
            Some(l) => Expr::List(l),
            None => Expr::Nil,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Expr::Nil)
    }

    /// Iterate this expression as a list (empty for nil and atoms).
    pub fn list_iter(&self) -> ListIter<'_> {
        match self {
            Expr::List(l) => l.iter(),
            _ => ListIter { next: None },
        }
    }

    pub fn list_len(&self) -> usize {
        self.list_iter().count()
    }

    /// n-th element when this is a list.
    pub fn nth(&self, n: usize) -> Option<&Expr> {
        self.list_iter().nth(n)
    }

    /// n-th cdr as an expression (`self` for 0, `Nil` past the end or for atoms).
    pub fn tail(&self, n: usize) -> &Expr {
        let mut cur = self;
        for _ in 0..n {
            cur = match cur {
                Expr::List(l) => l.cdr(),
                _ => return &Expr::Nil,
            };
        }
        cur
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_nil()
    }

    pub fn as_integer(&self) -> Option<i32> {
        match self {
            Expr::Integer(i) => Some(*i),
            Expr::Real(r) => Some(*r as i32),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            Expr::Integer(i) => Some(*i as f64),
            Expr::Real(r) => Some(*r),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Expr::String(s) => Some(s),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Expr::Symbol(s) => Some(s),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_list(&self) -> Option<&List> {
        match self {
            Expr::List(v) => Some(v),
            _ => None,
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Nil => write!(f, "nil"),
            Expr::T => write!(f, "T"),
            Expr::Integer(i) => write!(f, "{}", i),
            Expr::Real(r) => write!(f, "{}", r),
            Expr::String(s) => write!(f, "\"{}\"", s),
            Expr::Symbol(s) => write!(f, "{}", s),
            Expr::List(list) => {
                write!(f, "(")?;
                let mut cur = list;
                loop {
                    write!(f, "{}", cur.car())?;
                    match cur.cdr() {
                        Expr::List(next) => {
                            write!(f, " ")?;
                            cur = next;
                        }
                        Expr::Nil => break,
                        atom => {
                            write!(f, " . {}", atom)?;
                            break;
                        }
                    }
                }
                write!(f, ")")
            }
            Expr::Quote(e) => write!(f, "'{}", e),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn from_input(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        Parser::new(tokens)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(token)
    }

    pub fn parse_expr(&mut self) -> Option<Expr> {
        let token = self.advance()?.clone();
        match token {
            Token::LParen => {
                let mut items = Vec::new();
                let mut tail = Expr::Nil;
                loop {
                    match self.peek() {
                        None => break,
                        Some(Token::RParen) => {
                            self.advance();
                            break;
                        }
                        // dotted pair syntax: (a b . c)
                        Some(Token::Symbol(s)) if s == "." && !items.is_empty() => {
                            self.advance();
                            if let Some(expr) = self.parse_expr() {
                                tail = expr;
                            }
                            // consume up to and including the closing paren
                            while let Some(tok) = self.advance() {
                                if *tok == Token::RParen {
                                    break;
                                }
                            }
                            break;
                        }
                        _ => {
                            if let Some(expr) = self.parse_expr() {
                                items.push(expr);
                            }
                        }
                    }
                }
                let mut list = tail;
                for item in items.into_iter().rev() {
                    list = Expr::List(List::cons(item, list));
                }
                Some(list)
            }
            Token::RParen => None, // Unexpected, ignore
            Token::Quote => {
                let expr = self.parse_expr()?;
                Some(Expr::Quote(Box::new(expr)))
            }
            Token::Symbol(s) => Some(Expr::Symbol(s)),
            Token::String(s) => Some(Expr::String(s)),
            Token::Integer(i) => Some(Expr::Integer(i)),
            Token::Real(r) => Some(Expr::Real(r)),
            Token::Nil => Some(Expr::Nil),
            Token::T => Some(Expr::T),
        }
    }

    pub fn parse_all(&mut self) -> Vec<Expr> {
        let mut exprs = Vec::new();
        while self.pos < self.tokens.len() {
            if let Some(expr) = self.parse_expr() {
                exprs.push(expr);
            }
        }
        exprs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let mut parser = Parser::from_input("(+ 1 2)");
        let expr = parser.parse_expr().unwrap();
        if let Expr::List(items) = expr {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Expr::Symbol("+".to_string()));
            assert_eq!(items[1], Expr::Integer(1));
            assert_eq!(items[2], Expr::Integer(2));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_nested() {
        let mut parser = Parser::from_input("(defun square (x) (* x x))");
        let expr = parser.parse_expr().unwrap();
        if let Expr::List(items) = expr {
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], Expr::Symbol("DEFUN".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_dotted_pair() {
        let mut parser = Parser::from_input("(1 . 2) (a b . c) (x . nil)");
        let e = parser.parse_all();
        assert_eq!(e[0].to_string(), "(1 . 2)");
        assert_eq!(e[0].as_list().unwrap().cdr(), &Expr::Integer(2));
        assert_eq!(e[1].to_string(), "(A B . C)");
        assert_eq!(e[2].to_string(), "(X)");
    }

    #[test]
    fn test_parse_quote() {
        let mut parser = Parser::from_input("'(1 2 3)");
        let expr = parser.parse_expr().unwrap();
        assert!(matches!(expr, Expr::Quote(_)));
    }
}

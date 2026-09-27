//! Interned symbols and the builtin table.
//!
//! Every AutoLISP symbol is interned once, at parse time, into a `Sym`: a `u32`
//! id that compares, hashes and copies for free. The first `BUILTIN_COUNT` ids
//! are the builtins in the order of the `builtins!` table, so `Sym::builtin()`
//! resolves a call head to a `Builtin` variant with one comparison instead of a
//! chain of string compares, and variable frames can be small vectors keyed by id.
//! Symbol names are only needed for printing and for the foreign-function API.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, OnceLock};

macro_rules! builtins {
    ($($variant:ident => $name:literal,)*) => {
        /// Every function the interpreter implements natively. The first
        /// `SPECIAL_FORM_COUNT` variants receive their arguments unevaluated.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u32)]
        pub enum Builtin {
            $($variant,)*
        }

        pub const BUILTIN_NAMES: &[&str] = &[$($name,)*];

        impl Builtin {
            pub fn from_id(id: u32) -> Option<Builtin> {
                match id {
                    $(x if x == Builtin::$variant as u32 => Some(Builtin::$variant),)*
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                BUILTIN_NAMES[self as usize]
            }
        }
    };
}

builtins! {
    Quote => "QUOTE",
    Setq => "SETQ",
    Defun => "DEFUN",
    If => "IF",
    Cond => "COND",
    While => "WHILE",
    Repeat => "REPEAT",
    Progn => "PROGN",
    And => "AND",
    Or => "OR",
    Foreach => "FOREACH",
    Lambda => "LAMBDA",
    Mapcar => "MAPCAR",
    Add => "+",
    Sub => "-",
    Mul => "*",
    Div => "/",
    OnePlus => "1+",
    OneMinus => "1-",
    Abs => "ABS",
    Max => "MAX",
    Min => "MIN",
    Rem => "REM",
    Gcd => "GCD",
    Sin => "SIN",
    Cos => "COS",
    Atan => "ATAN",
    Sqrt => "SQRT",
    Expt => "EXPT",
    Exp => "EXP",
    Log => "LOG",
    Fix => "FIX",
    Float => "FLOAT",
    NumEq => "=",
    Neq => "/=",
    Lt => "<",
    Gt => ">",
    Le => "<=",
    Ge => ">=",
    Eq => "EQ",
    Equal => "EQUAL",
    Not => "NOT",
    Null => "NULL",
    Atom => "ATOM",
    Listp => "LISTP",
    Numberp => "NUMBERP",
    Minusp => "MINUSP",
    Zerop => "ZEROP",
    Type => "TYPE",
    Car => "CAR",
    Cdr => "CDR",
    Cadr => "CADR",
    Caddr => "CADDR",
    Caar => "CAAR",
    Cadar => "CADAR",
    Cddr => "CDDR",
    Cons => "CONS",
    List => "LIST",
    Append => "APPEND",
    Reverse => "REVERSE",
    Length => "LENGTH",
    Nth => "NTH",
    Last => "LAST",
    Member => "MEMBER",
    Assoc => "ASSOC",
    Subst => "SUBST",
    Strcat => "STRCAT",
    Strlen => "STRLEN",
    Substr => "SUBSTR",
    Strcase => "STRCASE",
    Ascii => "ASCII",
    Chr => "CHR",
    Atoi => "ATOI",
    Atof => "ATOF",
    Itoa => "ITOA",
    Rtos => "RTOS",
    Angtos => "ANGTOS",
    Read => "READ",
    Princ => "PRINC",
    Print => "PRINT",
    Prin1 => "PRIN1",
    Prompt => "PROMPT",
    Terpri => "TERPRI",
    Open => "OPEN",
    Close => "CLOSE",
    ReadLine => "READ-LINE",
    ReadChar => "READ-CHAR",
    WriteLine => "WRITE-LINE",
    WriteChar => "WRITE-CHAR",
    Findfile => "FINDFILE",
    Getint => "GETINT",
    Getreal => "GETREAL",
    Getstring => "GETSTRING",
    Getpoint => "GETPOINT",
    Getcorner => "GETCORNER",
    Getdist => "GETDIST",
    Getangle => "GETANGLE",
    Getkword => "GETKWORD",
    Initget => "INITGET",
    Command => "COMMAND",
    Getvar => "GETVAR",
    Setvar => "SETVAR",
    Distance => "DISTANCE",
    Angle => "ANGLE",
    Polar => "POLAR",
    Inters => "INTERS",
    Entsel => "ENTSEL",
    Entget => "ENTGET",
    Entmod => "ENTMOD",
    Entmake => "ENTMAKE",
    Entnext => "ENTNEXT",
    Entlast => "ENTLAST",
    KicadPin => "KICAD-PIN",
    KicadProp => "KICAD-PROP",
    KicadPad => "KICAD-PAD",
    Ssget => "SSGET",
    Sslength => "SSLENGTH",
    Ssname => "SSNAME",
    Ssadd => "SSADD",
    Ssdel => "SSDEL",
    Eval => "EVAL",
    Apply => "APPLY",
    Load => "LOAD",
    Alert => "ALERT",
    Quit => "QUIT",
    Exit => "EXIT",
}

/// Number of special forms at the start of the table (arguments unevaluated).
pub const SPECIAL_FORM_COUNT: u32 = 13;
pub const BUILTIN_COUNT: u32 = BUILTIN_NAMES.len() as u32;

struct Interner {
    ids: HashMap<String, u32>,
    names: Vec<String>,
}

fn interner() -> &'static Mutex<Interner> {
    static INTERNER: OnceLock<Mutex<Interner>> = OnceLock::new();
    INTERNER.get_or_init(|| {
        let mut ids = HashMap::new();
        let mut names = Vec::new();
        for (i, n) in BUILTIN_NAMES.iter().enumerate() {
            ids.insert((*n).to_string(), i as u32);
            names.push((*n).to_string());
        }
        Mutex::new(Interner { ids, names })
    })
}

/// An interned symbol. Names are uppercase (AutoLISP is case-insensitive).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

impl Sym {
    /// Intern a name exactly as given (callers uppercase first; the lexer does).
    pub fn intern(name: &str) -> Sym {
        let mut it = interner().lock().unwrap();
        if let Some(&id) = it.ids.get(name) {
            return Sym(id);
        }
        let id = it.names.len() as u32;
        it.names.push(name.to_string());
        it.ids.insert(name.to_string(), id);
        Sym(id)
    }

    /// Intern after uppercasing, for names coming from the host (`register_fn`, APIs).
    pub fn intern_upper(name: &str) -> Sym {
        Sym::intern(&name.to_uppercase())
    }

    pub fn id(self) -> u32 {
        self.0
    }

    /// The builtin this symbol names, if any.
    #[inline]
    pub fn builtin(self) -> Option<Builtin> {
        if self.0 < BUILTIN_COUNT {
            Builtin::from_id(self.0)
        } else {
            None
        }
    }

    pub const fn from_builtin(b: Builtin) -> Sym {
        Sym(b as u32)
    }

    /// The symbol's name (a copy; locks the interner).
    pub fn name(self) -> String {
        interner().lock().unwrap().names[self.0 as usize].clone()
    }
}

impl fmt::Display for Sym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

impl fmt::Debug for Sym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sym({}#{})", self.name(), self.0)
    }
}

impl PartialEq<str> for Sym {
    fn eq(&self, other: &str) -> bool {
        self.name() == other
    }
}

impl PartialEq<&str> for Sym {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_resolve_by_id() {
        assert_eq!(Sym::intern("SETQ").builtin(), Some(Builtin::Setq));
        assert_eq!(Sym::intern("+").builtin(), Some(Builtin::Add));
        assert_eq!(Sym::intern("KICAD-PIN").builtin(), Some(Builtin::KicadPin));
        assert_eq!(Sym::intern("MYFUNC").builtin(), None);
        assert_eq!(Sym::intern("MYFUNC"), Sym::intern("MYFUNC"));
        assert_ne!(Sym::intern("MYFUNC"), Sym::intern("MYFUNC2"));
        assert_eq!(Sym::intern("CAR").name(), "CAR");
        assert!((Builtin::Mapcar as u32) < SPECIAL_FORM_COUNT);
        assert!((Builtin::Add as u32) >= SPECIAL_FORM_COUNT);
    }
}

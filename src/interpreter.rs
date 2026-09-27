// AutoLISP Interpreter - AutoCAD 9/10 (DOS era) compatible
// Implements the subset of functions available in late 1980s AutoCAD

use crate::parser::{Expr, List};
use crate::compile::{compile_body, Node};
use crate::symbol::{Builtin, Sym, BUILTIN_COUNT};
use std::sync::OnceLock;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use smallvec::SmallVec;
use std::sync::Arc;

/// Context passed to foreign (extension) functions.
///
/// This avoids borrow conflicts with the Interpreter by only exposing
/// the mutable fields that extensions typically need.
pub struct ForeignContext<'a> {
    pub output: &'a mut Vec<String>,
    pub globals: &'a mut Globals,
    pub drawing: &'a mut DrawingState,
    /// Opaque user data set by the embedding application.
    /// Downcast to your application-specific state type.
    pub user_data: &'a mut dyn std::any::Any,
}

/// Type alias for foreign function closures.
pub type ForeignFn = Arc<dyn Fn(&mut ForeignContext, &[Expr]) -> Expr + Send + Sync>;

/// CAD type determines coordinate system and rendering behavior
/// - RustLisp: AutoCAD-style, Y increases upward, large coordinates (0-1000+)
/// - KiCad: Electronic symbol style, Y increases upward but centered at origin, small coords (-50 to 50)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(dead_code)]
pub enum CadType {
    #[default]
    RustLisp,
    KiCad,
}

// Simulated AutoCAD drawing entity
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DrawEntity {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: String,
    },
    Circle {
        cx: f64,
        cy: f64,
        radius: f64,
        layer: String,
    },
    Arc {
        cx: f64,
        cy: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        layer: String,
    },
    Text {
        x: f64,
        y: f64,
        height: f64,
        text: String,
        layer: String,
    },
    Point {
        x: f64,
        y: f64,
        layer: String,
    },
    Insert {
        block_name: String,
        x: f64,
        y: f64,
        scale: f64,
        rotation: f64,
        layer: String,
    },
    // KiCad specific entities
    Pin {
        name: String,
        number: String,
        etype: String,
        style: String,
        x: f64,
        y: f64,
        length: f64,
        rotation: f64,
        layer: String,
    },
    Property {
        key: String,
        value: String,
        x: f64,
        y: f64,
        rotation: f64,
        height: f64,
        visible: bool,
        layer: String,
    },
    Pad {
        name: String,
        ptype: String, // smd, thru_hole, etc
        shape: String, // rect, circle, oval, etc
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        drill: f64,
        rotation: f64,
        layers: String, // list of layers (e.g., "*.Cu" or "F.Cu,F.Paste")
    },
}

// Simulated AutoCAD drawing state
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct DrawingState {
    pub entities: Vec<DrawEntity>,
    pub current_layer: String,
    pub current_color: i32,
    pub current_point: Option<(f64, f64, f64)>,
    pub system_variables: HashMap<String, Expr>,
    pub cad_type: CadType,
}

impl DrawingState {
    pub fn new() -> Self {
        let mut state = DrawingState {
            entities: Vec::new(),
            current_layer: "0".to_string(),
            current_color: 7,
            current_point: None,
            system_variables: HashMap::new(),
            cad_type: CadType::default(),
        };
        // Initialize some common system variables
        state
            .system_variables
            .insert("CLAYER".to_string(), Expr::String("0".to_string()));
        state
            .system_variables
            .insert("CECOLOR".to_string(), Expr::Integer(7));
        state
    }
}

// File handle for AutoLISP file operations
#[derive(Debug)]
#[allow(dead_code)]
pub struct FileHandle {
    pub reader: Option<BufReader<File>>,
    pub writer: Option<File>,
    pub path: String,
}

// User-defined function
#[derive(Debug)]
pub struct UserFunction {
    pub params: Vec<Sym>,
    pub body: Vec<Expr>,
    /// The body compiled to a resolved tree on first call (see `compile.rs`).
    pub compiled: OnceLock<Vec<Node>>,
}

impl UserFunction {
    pub fn new(params: Vec<Sym>, body: Vec<Expr>) -> UserFunction {
        UserFunction { params, body, compiled: OnceLock::new() }
    }

    pub fn code(&self) -> &[Node] {
        self.compiled.get_or_init(|| compile_body(&self.params, &self.body))
    }
}

/// Maximum nesting of user-defined function calls (see `Interpreter::call_depth`).
/// Global variables, indexed by symbol id: a lookup is one bounds check and a
/// vector index, no hashing.
#[derive(Debug, Default)]
pub struct Globals {
    slots: Vec<Option<Expr>>,
}

impl Globals {
    pub fn get(&self, sym: Sym) -> Option<&Expr> {
        self.slots.get(sym.id() as usize).and_then(|s| s.as_ref())
    }

    pub fn set(&mut self, sym: Sym, value: Expr) {
        let i = sym.id() as usize;
        if i >= self.slots.len() {
            self.slots.resize(i + 1, None);
        }
        self.slots[i] = Some(value);
    }

    pub fn contains(&self, sym: Sym) -> bool {
        self.get(sym).is_some()
    }

    /// Convenience for hosts: by name (uppercased).
    pub fn get_named(&self, name: &str) -> Option<&Expr> {
        self.get(Sym::intern_upper(name))
    }

    pub fn set_named(&mut self, name: &str, value: Expr) {
        self.set(Sym::intern_upper(name), value)
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// User-defined functions indexed by symbol id, like `Globals`.
#[derive(Debug, Default)]
pub struct FunctionTable {
    slots: Vec<Option<Arc<UserFunction>>>,
}

impl FunctionTable {
    #[inline]
    pub fn get(&self, sym: Sym) -> Option<&Arc<UserFunction>> {
        self.slots.get(sym.id() as usize).and_then(|s| s.as_ref())
    }

    pub fn insert(&mut self, sym: Sym, f: Arc<UserFunction>) {
        let i = sym.id() as usize;
        if i >= self.slots.len() {
            self.slots.resize(i + 1, None);
        }
        self.slots[i] = Some(f);
    }

    pub fn contains_key(&self, sym: &Sym) -> bool {
        self.get(*sym).is_some()
    }

    pub fn contains_named(&self, name: &str) -> bool {
        self.contains_key(&Sym::intern_upper(name))
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }
}

/// One dynamic-scope frame: the parameters and locals of a running user
/// function. AutoLISP frames hold a handful of variables, so a linear scan over
/// interned ids beats a hash map and the vector is recycled between calls.
pub type Frame = Vec<(Sym, Expr)>;

/// Each Lisp-level call costs several Rust frames (eval -> eval_list ->
/// call_user_function -> builtin_if -> eval ...); 500 stays inside the 1 MB
/// default WASM stack and the 2 MB Rust test-thread stack with margin.
pub const MAX_CALL_DEPTH: usize = 500;

pub struct Interpreter {
    // Global symbol table
    pub globals: Globals,
    // User-defined functions, indexed by symbol id (one index per call);
    // Arc: a call only bumps a refcount instead of cloning params + body
    pub functions: FunctionTable,
    // Local scope stack (innermost last)
    pub locals: Vec<Frame>,
    // Recycled frame vectors, so a call in steady state allocates nothing
    frame_pool: Vec<Frame>,
    // Builtin names redefined by a user defun or a foreign function: checked
    // with one array index instead of a hash lookup on every builtin call
    shadowed: Vec<bool>,
    // Open file handles (AutoCAD 9/10 style - integer descriptors)
    pub file_handles: HashMap<i32, FileHandle>,
    next_handle: i32,
    // Current user-function call depth; guarded by MAX_CALL_DEPTH so runaway
    // recursion yields an AutoLISP error instead of overflowing the Rust stack
    // (a hard abort natively, and a crash of the whole page in WASM).
    call_depth: usize,
    // Simulated drawing state
    pub drawing: DrawingState,
    // Command queue for batch output
    pub command_log: Vec<String>,
    // Output buffer for PRINC/PRINT
    pub output: Vec<String>,
    // Foreign (extension) functions registered by embedding applications
    pub foreign_fns: HashMap<Sym, ForeignFn>,
    // Opaque user data for embedding applications
    pub user_data: Box<dyn std::any::Any + Send>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            globals: Globals::default(),
            functions: FunctionTable::default(),
            locals: Vec::new(),
            frame_pool: Vec::new(),
            shadowed: vec![false; BUILTIN_COUNT as usize],
            file_handles: HashMap::new(),
            next_handle: 1,
            call_depth: 0,
            drawing: DrawingState::new(),
            command_log: Vec::new(),
            output: Vec::new(),
            foreign_fns: HashMap::new(),
            user_data: Box::new(()),
        }
    }

    /// Register an external function callable from Lisp code.
    /// The name is uppercased to match AutoLISP convention.
    pub fn register_fn<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&mut ForeignContext, &[Expr]) -> Expr + Send + Sync + 'static,
    {
        let sym = Sym::intern_upper(name);
        if let Some(b) = sym.builtin() {
            self.shadowed[b as usize] = true;
        }
        self.foreign_fns.insert(sym, Arc::new(f));
    }

    #[inline]
    pub(crate) fn get_var(&self, sym: Sym) -> Expr {
        // Dynamic scoping: innermost frame first, then globals
        for frame in self.locals.iter().rev() {
            for (s, val) in frame.iter().rev() {
                if *s == sym {
                    return val.clone();
                }
            }
        }
        self.globals.get(sym).cloned().unwrap_or(Expr::Nil)
    }

    /// Get a variable by name (for hosts and tests).
    pub fn get_var_named(&self, name: &str) -> Expr {
        self.get_var(Sym::intern_upper(name))
    }

    #[inline]
    pub(crate) fn set_var(&mut self, sym: Sym, value: Expr) {
        // AutoLISP: setq assigns the innermost binding that exists (a parameter
        // or declared local of a running function); otherwise the global
        for frame in self.locals.iter_mut().rev() {
            for (s, val) in frame.iter_mut().rev() {
                if *s == sym {
                    *val = value;
                    return;
                }
            }
        }
        self.globals.set(sym, value);
    }

    /// Set a variable by name (for hosts and tests).
    pub fn set_var_named(&mut self, name: &str, value: Expr) {
        self.set_var(Sym::intern_upper(name), value)
    }

    pub fn eval(&mut self, expr: &Expr) -> Expr {
        match expr {
            Expr::Nil => Expr::Nil,
            Expr::T => Expr::T,
            Expr::Integer(i) => Expr::Integer(*i),
            Expr::Real(r) => Expr::Real(*r),
            Expr::String(s) => Expr::String(s.clone()),
            Expr::Symbol(name) => self.get_var(*name),
            Expr::Quote(e) => (**e).clone(),
            Expr::List(list) => self.eval_list(list),
        }
    }

    fn eval_list(&mut self, list: &List) -> Expr {
        let first = list.car();
        let head = match first {
            Expr::Symbol(name) => *name,
            _ => {
                // Could be a lambda or other callable - not in AutoCAD 9/10
                return Expr::Nil;
            }
        };

        // Unevaluated arguments: the rest of the call form (Nil or a List)
        let args = list.cdr();

        // Resolved once at parse time: builtin ids sit at the start of the symbol table
        let builtin = head.builtin();

        // Special forms (don't evaluate args)
        match builtin {
            Some(Builtin::Quote) => return args.nth(0).cloned().unwrap_or(Expr::Nil),
            Some(Builtin::Setq) => return self.builtin_setq(args),
            Some(Builtin::Defun) => return self.builtin_defun(args),
            Some(Builtin::If) => return self.builtin_if(args),
            Some(Builtin::Cond) => return self.builtin_cond(args),
            Some(Builtin::While) => return self.builtin_while(args),
            Some(Builtin::Repeat) => return self.builtin_repeat(args),
            Some(Builtin::Progn) => return self.builtin_progn(args),
            Some(Builtin::And) => return self.builtin_and(args),
            Some(Builtin::Or) => return self.builtin_or(args),
            Some(Builtin::Foreach) => return self.builtin_foreach(args),
            Some(Builtin::Mapcar) => return self.builtin_mapcar(args),
            Some(Builtin::Lambda) => return Expr::Nil, // Not really used in AutoCAD 9/10
            _ => {}
        }

        // Regular functions - evaluate args first (stack-allocated for up to 8)
        let evaled_args: SmallVec<[Expr; 8]> = args.list_iter().map(|a| self.eval(a)).collect();

        match builtin {
            // a user or foreign function may shadow a builtin name
            Some(b) if !self.shadowed[b as usize] => self.call_builtin(b, &evaled_args),
            _ => self.call_named(head, &evaled_args),
        }
    }

    #[inline]
    pub(crate) fn is_shadowed(&self, b: Builtin) -> bool {
        self.shadowed[b as usize]
    }

    /// Call a user-defined or foreign function by symbol, with evaluated arguments.
    pub(crate) fn call_named(&mut self, head: Sym, evaled_args: &[Expr]) -> Expr {
        if let Some(func) = self.functions.get(head).cloned() {
            return self.call_user_function(&func, evaled_args);
        }
        if let Some(f) = self.foreign_fns.get(&head).cloned() {
            let mut ctx = ForeignContext {
                output: &mut self.output,
                globals: &mut self.globals,
                drawing: &mut self.drawing,
                user_data: self.user_data.as_mut(),
            };
            return f(&mut ctx, evaled_args);
        }
        if let Some(b) = head.builtin() {
            // shadowed once, but the shadowing definition is gone again
            return self.call_builtin(b, evaled_args);
        }
        self.output
            .push(format!("Error: unknown function {}", head));
        Expr::Nil
    }

    /// Call a builtin with evaluated arguments (special forms return nil here).
    pub(crate) fn call_builtin(&mut self, builtin: Builtin, evaled_args: &[Expr]) -> Expr {
        // Fast path: two-integer arithmetic and comparison, the bulk of loop
        // and recursion bodies (wrapping, like AutoLISP's 32-bit integers)
        if let [Expr::Integer(a), Expr::Integer(b)] = evaled_args {
            let (a, b) = (*a, *b);
            match builtin {
                Builtin::Add => return Expr::Integer(a.wrapping_add(b)),
                Builtin::Sub => return Expr::Integer(a.wrapping_sub(b)),
                Builtin::Mul => return Expr::Integer(a.wrapping_mul(b)),
                Builtin::Lt => return if a < b { Expr::T } else { Expr::Nil },
                Builtin::Gt => return if a > b { Expr::T } else { Expr::Nil },
                Builtin::Le => return if a <= b { Expr::T } else { Expr::Nil },
                Builtin::Ge => return if a >= b { Expr::T } else { Expr::Nil },
                Builtin::NumEq => return if a == b { Expr::T } else { Expr::Nil },
                Builtin::Neq => return if a != b { Expr::T } else { Expr::Nil },
                _ => {}
            }
        }
        match builtin {
            // Arithmetic
            Builtin::Add => self.builtin_add(evaled_args),
            Builtin::Sub => self.builtin_sub(evaled_args),
            Builtin::Mul => self.builtin_mul(evaled_args),
            Builtin::Div => self.builtin_div(evaled_args),
            Builtin::OnePlus => self.builtin_1plus(evaled_args),
            Builtin::OneMinus => self.builtin_1minus(evaled_args),
            Builtin::Abs => self.builtin_abs(evaled_args),
            Builtin::Max => self.builtin_max(evaled_args),
            Builtin::Min => self.builtin_min(evaled_args),
            Builtin::Rem => self.builtin_rem(evaled_args),
            Builtin::Gcd => self.builtin_gcd(evaled_args),
            Builtin::Sin => self.builtin_sin(evaled_args),
            Builtin::Cos => self.builtin_cos(evaled_args),
            Builtin::Atan => self.builtin_atan(evaled_args),
            Builtin::Sqrt => self.builtin_sqrt(evaled_args),
            Builtin::Expt => self.builtin_expt(evaled_args),
            Builtin::Exp => self.builtin_exp(evaled_args),
            Builtin::Log => self.builtin_log(evaled_args),
            Builtin::Fix => self.builtin_fix(evaled_args),
            Builtin::Float => self.builtin_float(evaled_args),

            // Comparison
            Builtin::NumEq => self.builtin_eq(evaled_args),
            Builtin::Neq => self.builtin_neq(evaled_args),
            Builtin::Lt => self.builtin_lt(evaled_args),
            Builtin::Gt => self.builtin_gt(evaled_args),
            Builtin::Le => self.builtin_le(evaled_args),
            Builtin::Ge => self.builtin_ge(evaled_args),
            Builtin::Eq => self.builtin_eq_sym(evaled_args),
            Builtin::Equal => self.builtin_equal(evaled_args),

            // Logical
            Builtin::Not => self.builtin_not(evaled_args),
            Builtin::Null => self.builtin_null(evaled_args),

            // Type predicates
            Builtin::Atom => self.builtin_atom(evaled_args),
            Builtin::Listp => self.builtin_listp(evaled_args),
            Builtin::Numberp => self.builtin_numberp(evaled_args),
            Builtin::Minusp => self.builtin_minusp(evaled_args),
            Builtin::Zerop => self.builtin_zerop(evaled_args),
            Builtin::Type => self.builtin_type(evaled_args),

            // List operations
            Builtin::Car => self.builtin_car(evaled_args),
            Builtin::Cdr => self.builtin_cdr(evaled_args),
            Builtin::Cadr => self.builtin_cadr(evaled_args),
            Builtin::Caddr => self.builtin_caddr(evaled_args),
            Builtin::Caar => self.builtin_caar(evaled_args),
            Builtin::Cadar => self.builtin_cadar(evaled_args),
            Builtin::Cddr => self.builtin_cddr(evaled_args),
            Builtin::Cons => self.builtin_cons(evaled_args),
            Builtin::List => self.builtin_list(evaled_args),
            Builtin::Append => self.builtin_append(evaled_args),
            Builtin::Reverse => self.builtin_reverse(evaled_args),
            Builtin::Length => self.builtin_length(evaled_args),
            Builtin::Nth => self.builtin_nth(evaled_args),
            Builtin::Last => self.builtin_last(evaled_args),
            Builtin::Member => self.builtin_member(evaled_args),
            Builtin::Assoc => self.builtin_assoc(evaled_args),
            Builtin::Subst => self.builtin_subst(evaled_args),

            // String operations
            Builtin::Strcat => self.builtin_strcat(evaled_args),
            Builtin::Strlen => self.builtin_strlen(evaled_args),
            Builtin::Substr => self.builtin_substr(evaled_args),
            Builtin::Strcase => self.builtin_strcase(evaled_args),
            Builtin::Ascii => self.builtin_ascii(evaled_args),
            Builtin::Chr => self.builtin_chr(evaled_args),
            Builtin::Atoi => self.builtin_atoi(evaled_args),
            Builtin::Atof => self.builtin_atof(evaled_args),
            Builtin::Itoa => self.builtin_itoa(evaled_args),
            Builtin::Rtos => self.builtin_rtos(evaled_args),
            Builtin::Angtos => self.builtin_angtos(evaled_args),
            Builtin::Read => self.builtin_read_str(evaled_args),

            // I/O
            Builtin::Princ => self.builtin_princ(evaled_args),
            Builtin::Print => self.builtin_print(evaled_args),
            Builtin::Prin1 => self.builtin_prin1(evaled_args),
            Builtin::Prompt => self.builtin_prompt(evaled_args),
            Builtin::Terpri => self.builtin_terpri(evaled_args),

            // File I/O (DOS style)
            Builtin::Open => self.builtin_open(evaled_args),
            Builtin::Close => self.builtin_close(evaled_args),
            Builtin::ReadLine => self.builtin_read_line(evaled_args),
            Builtin::ReadChar => self.builtin_read_char(evaled_args),
            Builtin::WriteLine => self.builtin_write_line(evaled_args),
            Builtin::WriteChar => self.builtin_write_char(evaled_args),
            Builtin::Findfile => self.builtin_findfile(evaled_args),

            // User input (simulated - returns nil in batch mode)
            Builtin::Getint => Expr::Nil,
            Builtin::Getreal => Expr::Nil,
            Builtin::Getstring => Expr::Nil,
            Builtin::Getpoint => Expr::Nil,
            Builtin::Getcorner => Expr::Nil,
            Builtin::Getdist => Expr::Nil,
            Builtin::Getangle => Expr::Nil,
            Builtin::Getkword => Expr::Nil,
            Builtin::Initget => Expr::Nil,

            // AutoCAD specific
            Builtin::Command => self.builtin_command(evaled_args),
            Builtin::Getvar => self.builtin_getvar(evaled_args),
            Builtin::Setvar => self.builtin_setvar(evaled_args),

            // Geometry (points as lists)
            Builtin::Distance => self.builtin_distance(evaled_args),
            Builtin::Angle => self.builtin_angle(evaled_args),
            Builtin::Polar => self.builtin_polar(evaled_args),
            Builtin::Inters => self.builtin_inters(evaled_args),

            // Entity access (simulated)
            Builtin::Entsel => Expr::Nil,
            Builtin::Entget => Expr::Nil,
            Builtin::Entmod => Expr::Nil,
            Builtin::Entmake => self.builtin_entmake(evaled_args),
            Builtin::Entnext => Expr::Nil,
            Builtin::Entlast => Expr::Nil,

            // KiCad extensions
            Builtin::KicadPin => self.builtin_kicad_pin(evaled_args),
            Builtin::KicadProp => self.builtin_kicad_prop(evaled_args),
            Builtin::KicadPad => self.builtin_kicad_pad(evaled_args),

            // Selection sets (simulated)
            Builtin::Ssget => Expr::Nil,
            Builtin::Sslength => Expr::Integer(0),
            Builtin::Ssname => Expr::Nil,
            Builtin::Ssadd => Expr::Nil,
            Builtin::Ssdel => Expr::Nil,

            // Misc
            Builtin::Eval => {
                if let Some(e) = evaled_args.first() {
                    self.eval(e)
                } else {
                    Expr::Nil
                }
            }
            Builtin::Apply => self.builtin_apply(evaled_args),
            Builtin::Load => self.builtin_load(evaled_args),
            Builtin::Alert => self.builtin_alert(evaled_args),
            Builtin::Quit => {
                self.output.push("*** QUIT ***".to_string());
                Expr::Nil
            }
            Builtin::Exit => {
                self.output.push("*** EXIT ***".to_string());
                Expr::Nil
            }

            // special forms were handled above
            Builtin::Quote
            | Builtin::Setq
            | Builtin::Defun
            | Builtin::If
            | Builtin::Cond
            | Builtin::While
            | Builtin::Repeat
            | Builtin::Progn
            | Builtin::And
            | Builtin::Or
            | Builtin::Foreach
            | Builtin::Mapcar
            | Builtin::Lambda => Expr::Nil,
        }
    }

    pub(crate) fn call_user_function(&mut self, func: &UserFunction, args: &[Expr]) -> Expr {
        if self.call_depth >= MAX_CALL_DEPTH {
            self.output.push(format!(
                "Error: recursion too deep (limit {MAX_CALL_DEPTH} nested calls)"
            ));
            return Expr::Nil;
        }
        self.call_depth += 1;
        // New frame from the pool: parameters first, missing ones are nil
        let mut frame = self.frame_pool.pop().unwrap_or_default();
        for (i, param) in func.params.iter().enumerate() {
            let val = args.get(i).cloned().unwrap_or(Expr::Nil);
            frame.push((*param, val));
        }
        self.locals.push(frame);

        // Run the body compiled on first call
        let mut result = Expr::Nil;
        for node in func.code() {
            result = node.run(self);
        }

        // Pop scope and recycle the vector
        if let Some(mut frame) = self.locals.pop() {
            frame.clear();
            self.frame_pool.push(frame);
        }
        self.call_depth -= 1;
        result
    }

    // Special forms

    fn builtin_setq(&mut self, args: &Expr) -> Expr {
        let mut result = Expr::Nil;
        let mut rest = args;
        while let (Some(sym), Some(value_expr)) = (rest.nth(0), rest.nth(1)) {
            if let Expr::Symbol(name) = sym {
                let val = self.eval(value_expr);
                self.set_var(*name, val.clone());
                result = val;
            }
            rest = rest.tail(2);
        }
        result
    }

    pub(crate) fn builtin_defun(&mut self, args: &Expr) -> Expr {
        if args.list_len() < 3 {
            return Expr::Nil;
        }
        let name = match args.nth(0) {
            Some(Expr::Symbol(s)) => *s,
            _ => return Expr::Nil,
        };
        let slash = Sym::from_builtin(Builtin::Div);
        let params = match args.nth(1) {
            Some(Expr::List(items)) => {
                items
                    .iter()
                    .filter_map(|e| {
                        if let Expr::Symbol(s) = e {
                            // Skip / which separates required from local vars in AutoLISP
                            if *s != slash {
                                Some(*s)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let body: Vec<Expr> = args.tail(2).list_iter().cloned().collect();
        if let Some(b) = name.builtin() {
            self.shadowed[b as usize] = true;
        }
        self.functions
            .insert(name, Arc::new(UserFunction::new(params, body)));
        Expr::Symbol(name)
    }

    fn builtin_if(&mut self, args: &Expr) -> Expr {
        let Some(test) = args.nth(0) else {
            return Expr::Nil;
        };
        let cond = self.eval(test);
        if cond.is_truthy() {
            args.nth(1).map(|e| self.eval(e)).unwrap_or(Expr::Nil)
        } else {
            args.nth(2).map(|e| self.eval(e)).unwrap_or(Expr::Nil)
        }
    }

    fn builtin_cond(&mut self, args: &Expr) -> Expr {
        for clause in args.list_iter() {
            if let Expr::List(items) = clause {
                let test = self.eval(items.car());
                if test.is_truthy() {
                    let mut result = test;
                    for expr in items.cdr().list_iter() {
                        result = self.eval(expr);
                    }
                    return result;
                }
            }
        }
        Expr::Nil
    }

    fn builtin_while(&mut self, args: &Expr) -> Expr {
        let Some(test) = args.nth(0) else {
            return Expr::Nil;
        };
        let body = args.tail(1);
        let mut result = Expr::Nil;
        while self.eval(test).is_truthy() {
            for expr in body.list_iter() {
                result = self.eval(expr);
            }
        }
        result
    }

    fn builtin_repeat(&mut self, args: &Expr) -> Expr {
        let Some(count_expr) = args.nth(0) else {
            return Expr::Nil;
        };
        let count = self.eval(count_expr).as_integer().unwrap_or(0);
        let body = args.tail(1);
        let mut result = Expr::Nil;
        for _ in 0..count {
            for expr in body.list_iter() {
                result = self.eval(expr);
            }
        }
        result
    }

    fn builtin_progn(&mut self, args: &Expr) -> Expr {
        let mut result = Expr::Nil;
        for expr in args.list_iter() {
            result = self.eval(expr);
        }
        result
    }

    fn builtin_and(&mut self, args: &Expr) -> Expr {
        for arg in args.list_iter() {
            let val = self.eval(arg);
            if !val.is_truthy() {
                return Expr::Nil;
            }
        }
        Expr::T
    }

    fn builtin_or(&mut self, args: &Expr) -> Expr {
        for arg in args.list_iter() {
            let val = self.eval(arg);
            if val.is_truthy() {
                return val;
            }
        }
        Expr::Nil
    }

    fn builtin_foreach(&mut self, args: &Expr) -> Expr {
        let (Some(var), Some(list_expr)) = (args.nth(0), args.nth(1)) else {
            return Expr::Nil;
        };
        let var_name = match var {
            Expr::Symbol(s) => *s,
            _ => return Expr::Nil,
        };
        let list = self.eval(list_expr);
        let items = match list {
            Expr::List(v) => v,
            _ => return Expr::Nil,
        };
        let body = args.tail(2);

        let mut result = Expr::Nil;
        for item in items.iter() {
            self.set_var(var_name, item.clone());
            for expr in body.list_iter() {
                result = self.eval(expr);
            }
        }
        result
    }

    // Arithmetic functions

    fn builtin_add(&self, args: &[Expr]) -> Expr {
        let mut sum_int: i32 = 0;
        let mut sum_real: f64 = 0.0;
        let mut is_real = false;

        for arg in args {
            match arg {
                Expr::Integer(i) => sum_int += i,
                Expr::Real(r) => {
                    is_real = true;
                    sum_real += r;
                }
                _ => {}
            }
        }

        if is_real {
            Expr::Real(sum_int as f64 + sum_real)
        } else {
            Expr::Integer(sum_int)
        }
    }

    fn builtin_sub(&self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Integer(0);
        }
        let first = args[0].as_real().unwrap_or(0.0);
        let is_real = matches!(args[0], Expr::Real(_));

        if args.len() == 1 {
            if is_real {
                return Expr::Real(-first);
            } else {
                return Expr::Integer(-first as i32);
            }
        }

        let mut result = first;
        let mut any_real = is_real;
        for arg in &args[1..] {
            if matches!(arg, Expr::Real(_)) {
                any_real = true;
            }
            result -= arg.as_real().unwrap_or(0.0);
        }

        if any_real {
            Expr::Real(result)
        } else {
            Expr::Integer(result as i32)
        }
    }

    fn builtin_mul(&self, args: &[Expr]) -> Expr {
        let mut prod: f64 = 1.0;
        let mut is_real = false;

        for arg in args {
            if matches!(arg, Expr::Real(_)) {
                is_real = true;
            }
            prod *= arg.as_real().unwrap_or(1.0);
        }

        if is_real {
            Expr::Real(prod)
        } else {
            Expr::Integer(prod as i32)
        }
    }

    fn builtin_div(&self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Integer(1);
        }
        let first = args[0].as_real().unwrap_or(1.0);
        let is_real = matches!(args[0], Expr::Real(_));

        if args.len() == 1 {
            return args[0].clone();
        }

        let mut result = first;
        let mut any_real = is_real;
        for arg in &args[1..] {
            if matches!(arg, Expr::Real(_)) {
                any_real = true;
            }
            let divisor = arg.as_real().unwrap_or(1.0);
            if divisor != 0.0 {
                result /= divisor;
            }
        }

        if any_real {
            Expr::Real(result)
        } else {
            Expr::Integer(result as i32)
        }
    }

    fn builtin_1plus(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(i)) => Expr::Integer(i + 1),
            Some(Expr::Real(r)) => Expr::Real(r + 1.0),
            _ => Expr::Nil,
        }
    }

    fn builtin_1minus(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(i)) => Expr::Integer(i - 1),
            Some(Expr::Real(r)) => Expr::Real(r - 1.0),
            _ => Expr::Nil,
        }
    }

    fn builtin_abs(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(i)) => Expr::Integer(i.abs()),
            Some(Expr::Real(r)) => Expr::Real(r.abs()),
            _ => Expr::Nil,
        }
    }

    fn builtin_max(&self, args: &[Expr]) -> Expr {
        let mut max: Option<f64> = None;
        let mut is_real = false;
        for arg in args {
            if matches!(arg, Expr::Real(_)) {
                is_real = true;
            }
            if let Some(v) = arg.as_real() {
                max = Some(max.map(|m| m.max(v)).unwrap_or(v));
            }
        }
        match max {
            Some(v) if is_real => Expr::Real(v),
            Some(v) => Expr::Integer(v as i32),
            None => Expr::Nil,
        }
    }

    fn builtin_min(&self, args: &[Expr]) -> Expr {
        let mut min: Option<f64> = None;
        let mut is_real = false;
        for arg in args {
            if matches!(arg, Expr::Real(_)) {
                is_real = true;
            }
            if let Some(v) = arg.as_real() {
                min = Some(min.map(|m| m.min(v)).unwrap_or(v));
            }
        }
        match min {
            Some(v) if is_real => Expr::Real(v),
            Some(v) => Expr::Integer(v as i32),
            None => Expr::Nil,
        }
    }

    fn builtin_rem(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let a = args[0].as_integer().unwrap_or(0);
        let b = args[1].as_integer().unwrap_or(1);
        if b == 0 {
            Expr::Nil
        } else {
            Expr::Integer(a % b)
        }
    }

    fn builtin_gcd(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let mut a = args[0].as_integer().unwrap_or(0).abs();
        let mut b = args[1].as_integer().unwrap_or(0).abs();
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        Expr::Integer(a)
    }

    fn builtin_sin(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Real(r.sin()))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_cos(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Real(r.cos()))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_atan(&self, args: &[Expr]) -> Expr {
        match (args.first(), args.get(1)) {
            (Some(y), Some(x)) => {
                let y = y.as_real().unwrap_or(0.0);
                let x = x.as_real().unwrap_or(1.0);
                Expr::Real(y.atan2(x))
            }
            (Some(v), None) => Expr::Real(v.as_real().unwrap_or(0.0).atan()),
            _ => Expr::Nil,
        }
    }

    fn builtin_sqrt(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Real(r.sqrt()))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_expt(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let base = args[0].as_real().unwrap_or(0.0);
        let exp = args[1].as_real().unwrap_or(1.0);
        Expr::Real(base.powf(exp))
    }

    fn builtin_exp(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Real(r.exp()))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_log(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Real(r.ln()))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_fix(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(|r| Expr::Integer(r as i32))
            .unwrap_or(Expr::Nil)
    }

    fn builtin_float(&self, args: &[Expr]) -> Expr {
        args.first()
            .and_then(|e| e.as_real())
            .map(Expr::Real)
            .unwrap_or(Expr::Nil)
    }

    // Comparison functions

    fn builtin_eq(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::T;
        }
        let a = args[0].as_real();
        let b = args[1].as_real();
        match (a, b) {
            (Some(a), Some(b)) if (a - b).abs() < 1e-10 => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_neq(&self, args: &[Expr]) -> Expr {
        if self.builtin_eq(args).is_truthy() {
            Expr::Nil
        } else {
            Expr::T
        }
    }

    fn builtin_lt(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let a = args[0].as_real().unwrap_or(0.0);
        let b = args[1].as_real().unwrap_or(0.0);
        if a < b {
            Expr::T
        } else {
            Expr::Nil
        }
    }

    fn builtin_gt(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let a = args[0].as_real().unwrap_or(0.0);
        let b = args[1].as_real().unwrap_or(0.0);
        if a > b {
            Expr::T
        } else {
            Expr::Nil
        }
    }

    fn builtin_le(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let a = args[0].as_real().unwrap_or(0.0);
        let b = args[1].as_real().unwrap_or(0.0);
        if a <= b {
            Expr::T
        } else {
            Expr::Nil
        }
    }

    fn builtin_ge(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let a = args[0].as_real().unwrap_or(0.0);
        let b = args[1].as_real().unwrap_or(0.0);
        if a >= b {
            Expr::T
        } else {
            Expr::Nil
        }
    }

    fn builtin_eq_sym(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        // EQ checks if same object (for symbols, same name)
        match (&args[0], &args[1]) {
            (Expr::Symbol(a), Expr::Symbol(b)) if a == b => Expr::T,
            (Expr::Nil, Expr::Nil) => Expr::T,
            (Expr::T, Expr::T) => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_equal(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        if args[0] == args[1] {
            Expr::T
        } else {
            Expr::Nil
        }
    }

    // Logical

    fn builtin_not(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(e) if e.is_truthy() => Expr::Nil,
            _ => Expr::T,
        }
    }

    fn builtin_null(&self, args: &[Expr]) -> Expr {
        self.builtin_not(args)
    }

    // Type predicates

    fn builtin_atom(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(_)) => Expr::Nil,
            _ => Expr::T,
        }
    }

    fn builtin_listp(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(_)) => Expr::T,
            Some(Expr::Nil) => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_numberp(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(_)) | Some(Expr::Real(_)) => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_minusp(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(i)) if *i < 0 => Expr::T,
            Some(Expr::Real(r)) if *r < 0.0 => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_zerop(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(0)) => Expr::T,
            Some(Expr::Real(r)) if r.abs() < 1e-10 => Expr::T,
            _ => Expr::Nil,
        }
    }

    fn builtin_type(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::Integer(_)) => Expr::Symbol(Sym::intern("INT")),
            Some(Expr::Real(_)) => Expr::Symbol(Sym::intern("REAL")),
            Some(Expr::String(_)) => Expr::Symbol(Sym::intern("STR")),
            Some(Expr::Symbol(_)) => Expr::Symbol(Sym::intern("SYM")),
            Some(Expr::List(_)) => Expr::Symbol(Sym::intern("LIST")),
            Some(Expr::Nil) | None => Expr::Nil,
            Some(Expr::T) => Expr::Symbol(Sym::intern("SYM")),
            Some(Expr::Quote(_)) => Expr::Symbol(Sym::intern("LIST")),
        }
    }

    // List operations

    fn builtin_car(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.car().clone(),
            _ => Expr::Nil,
        }
    }

    fn builtin_cdr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.cdr().clone(),
            _ => Expr::Nil,
        }
    }

    fn builtin_cadr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.get(1).cloned().unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_caddr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.get(2).cloned().unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_caar(&self, args: &[Expr]) -> Expr {
        let car = self.builtin_car(args);
        self.builtin_car(&[car])
    }

    fn builtin_cadar(&self, args: &[Expr]) -> Expr {
        let car = self.builtin_car(args);
        self.builtin_cadr(&[car])
    }

    fn builtin_cddr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.cdr().tail(1).clone(),
            _ => Expr::Nil,
        }
    }

    fn builtin_cons(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        // O(1): one new cell sharing the tail; an atom tail makes a dotted pair (a . b)
        Expr::List(List::cons(args[0].clone(), args[1].clone()))
    }

    fn builtin_list(&self, args: &[Expr]) -> Expr {
        Expr::list(args.to_vec())
    }

    fn builtin_append(&self, args: &[Expr]) -> Expr {
        // copies all but the last list, which is shared (classic append)
        let mut result = args.last().cloned().unwrap_or(Expr::Nil);
        if !matches!(result, Expr::List(_) | Expr::Nil) {
            result = Expr::Nil;
        }
        for arg in args.iter().rev().skip(1) {
            if let Expr::List(v) = arg {
                for item in v.to_vec().into_iter().rev() {
                    result = Expr::List(List::cons(item, result));
                }
            }
        }
        result
    }

    fn builtin_reverse(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => {
                let mut rev = Expr::Nil;
                for item in v.iter() {
                    rev = Expr::List(List::cons(item.clone(), rev));
                }
                rev
            }
            _ => Expr::Nil,
        }
    }

    fn builtin_length(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => Expr::Integer(v.len() as i32),
            Some(Expr::String(s)) => Expr::Integer(s.len() as i32),
            _ => Expr::Integer(0),
        }
    }

    fn builtin_nth(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let n = args[0].as_integer().unwrap_or(0) as usize;
        match &args[1] {
            Expr::List(v) => v.get(n).cloned().unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_last(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.last().cloned().unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_member(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let item = &args[0];
        let mut cur = &args[1];
        while let Expr::List(cell) = cur {
            if cell.car() == item {
                return cur.clone();
            }
            cur = cell.cdr();
        }
        Expr::Nil
    }

    fn builtin_assoc(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let key = &args[0];
        if let Expr::List(alist) = &args[1] {
            for item in alist.iter() {
                if let Expr::List(pair) = item {
                    if let Some(first) = pair.first() {
                        if first == key {
                            return item.clone();
                        }
                    }
                }
            }
        }
        Expr::Nil
    }

    fn builtin_subst(&self, args: &[Expr]) -> Expr {
        if args.len() < 3 {
            return Expr::Nil;
        }
        let new_item = &args[0];
        let old_item = &args[1];
        match &args[2] {
            Expr::List(v) => {
                let result: Vec<Expr> = v
                    .iter()
                    .map(|e| {
                        if e == old_item {
                            new_item.clone()
                        } else {
                            e.clone()
                        }
                    })
                    .collect();
                Expr::list(result)
            }
            other => other.clone(),
        }
    }

    pub(crate) fn builtin_mapcar(&mut self, args: &Expr) -> Expr {
        let (Some(func_expr), Some(list_expr)) = (args.nth(0), args.nth(1)) else {
            return Expr::Nil;
        };
        let func_name = match func_expr {
            Expr::Quote(e) => match e.as_ref() {
                Expr::Symbol(s) => *s,
                _ => return Expr::Nil,
            },
            Expr::Symbol(s) => *s,
            _ => return Expr::Nil,
        };
        let list = self.eval(list_expr);
        if let Expr::List(items) = list {
            let results: Vec<Expr> = items
                .iter()
                .map(|item| {
                    // items are values already: quote them so a list element is not re-evaluated as a call
                    let call = Expr::list(vec![
                        Expr::Symbol(func_name),
                        Expr::Quote(Box::new(item.clone())),
                    ]);
                    self.eval(&call)
                })
                .collect();
            Expr::list(results)
        } else {
            Expr::Nil
        }
    }

    // String operations

    fn builtin_strcat(&self, args: &[Expr]) -> Expr {
        let mut result = String::new();
        for arg in args {
            if let Expr::String(s) = arg {
                result.push_str(s);
            }
        }
        Expr::String(result)
    }

    fn builtin_strlen(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(s)) => Expr::Integer(s.len() as i32),
            _ => Expr::Integer(0),
        }
    }

    fn builtin_substr(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let s = match &args[0] {
            Expr::String(s) => s,
            _ => return Expr::Nil,
        };
        let start = args[1].as_integer().unwrap_or(1).max(1) as usize - 1;
        let len = args
            .get(2)
            .and_then(|e| e.as_integer())
            .map(|l| l as usize)
            .unwrap_or(s.len());
        let end = (start + len).min(s.len());
        if start < s.len() {
            Expr::String(s[start..end].to_string())
        } else {
            Expr::String(String::new())
        }
    }

    fn builtin_strcase(&self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Nil;
        }
        let s = match &args[0] {
            Expr::String(s) => s,
            _ => return Expr::Nil,
        };
        let upper = args.get(1).map(|e| e.is_truthy()).unwrap_or(false);
        if upper {
            Expr::String(s.to_lowercase())
        } else {
            Expr::String(s.to_uppercase())
        }
    }

    fn builtin_ascii(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(s)) => s
                .chars()
                .next()
                .map(|c| Expr::Integer(c as i32))
                .unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_chr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(e) => {
                let code = e.as_integer().unwrap_or(0);
                if (0..=127).contains(&code) {
                    Expr::String((code as u8 as char).to_string())
                } else {
                    Expr::Nil
                }
            }
            None => Expr::Nil,
        }
    }

    fn builtin_atoi(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(s)) => s
                .trim()
                .parse::<i32>()
                .map(Expr::Integer)
                .unwrap_or(Expr::Integer(0)),
            _ => Expr::Integer(0),
        }
    }

    fn builtin_atof(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(s)) => s
                .trim()
                .parse::<f64>()
                .map(Expr::Real)
                .unwrap_or(Expr::Real(0.0)),
            _ => Expr::Real(0.0),
        }
    }

    fn builtin_itoa(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(e) => Expr::String(e.as_integer().unwrap_or(0).to_string()),
            None => Expr::String("0".to_string()),
        }
    }

    fn builtin_rtos(&self, args: &[Expr]) -> Expr {
        let num = args.first().and_then(|e| e.as_real()).unwrap_or(0.0);
        let mode = args.get(1).and_then(|e| e.as_integer()).unwrap_or(2);
        let prec = args.get(2).and_then(|e| e.as_integer()).unwrap_or(4) as usize;

        match mode {
            1 => Expr::String(format!("{:.prec$e}", num, prec = prec)),
            2 => Expr::String(format!("{:.prec$}", num, prec = prec)),
            _ => Expr::String(format!("{}", num)),
        }
    }

    fn builtin_angtos(&self, args: &[Expr]) -> Expr {
        let angle = args.first().and_then(|e| e.as_real()).unwrap_or(0.0);
        let mode = args.get(1).and_then(|e| e.as_integer()).unwrap_or(0);
        let prec = args.get(2).and_then(|e| e.as_integer()).unwrap_or(4) as usize;

        let degrees = angle.to_degrees();
        match mode {
            0 => Expr::String(format!("{:.prec$}", degrees, prec = prec)),
            1 => {
                let d = degrees.floor() as i32;
                let m = ((degrees - d as f64) * 60.0).floor() as i32;
                let s = (degrees - d as f64 - m as f64 / 60.0) * 3600.0;
                Expr::String(format!("{}d{}'{:.prec$}\"", d, m, s, prec = prec))
            }
            _ => Expr::String(format!("{:.prec$}", angle, prec = prec)),
        }
    }

    fn builtin_read_str(&mut self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(s)) => {
                let mut parser = crate::parser::Parser::from_input(s);
                parser.parse_expr().unwrap_or(Expr::Nil)
            }
            _ => Expr::Nil,
        }
    }

    // I/O

    fn builtin_princ(&mut self, args: &[Expr]) -> Expr {
        if let Some(arg) = args.first() {
            let s = match arg {
                Expr::String(s) => s.clone(),
                other => format!("{}", other),
            };
            self.output.push(s);
            return arg.clone();
        }
        Expr::Nil
    }

    fn builtin_print(&mut self, args: &[Expr]) -> Expr {
        self.output.push(String::new()); // newline before
        if let Some(arg) = args.first() {
            self.output.push(format!("{}", arg));
            self.output.push(" ".to_string());
            return arg.clone();
        }
        Expr::Nil
    }

    fn builtin_prin1(&mut self, args: &[Expr]) -> Expr {
        if let Some(arg) = args.first() {
            self.output.push(format!("{}", arg));
            return arg.clone();
        }
        Expr::Nil
    }

    fn builtin_prompt(&mut self, args: &[Expr]) -> Expr {
        if let Some(Expr::String(s)) = args.first() {
            self.output.push(s.clone());
        }
        Expr::Nil
    }

    fn builtin_terpri(&mut self, _args: &[Expr]) -> Expr {
        self.output.push(String::new());
        Expr::Nil
    }

    // File I/O

    fn builtin_open(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let path = match &args[0] {
            Expr::String(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let mode = match &args[1] {
            Expr::String(s) => s.to_uppercase(),
            _ => return Expr::Nil,
        };

        let handle = self.next_handle;
        self.next_handle += 1;

        match mode.as_str() {
            "R" => match File::open(&path) {
                Ok(f) => {
                    self.file_handles.insert(
                        handle,
                        FileHandle {
                            reader: Some(BufReader::new(f)),
                            writer: None,
                            path,
                        },
                    );
                    Expr::Integer(handle)
                }
                Err(_) => Expr::Nil,
            },
            "W" => match File::create(&path) {
                Ok(f) => {
                    self.file_handles.insert(
                        handle,
                        FileHandle {
                            reader: None,
                            writer: Some(f),
                            path,
                        },
                    );
                    Expr::Integer(handle)
                }
                Err(_) => Expr::Nil,
            },
            "A" => {
                match std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                {
                    Ok(f) => {
                        self.file_handles.insert(
                            handle,
                            FileHandle {
                                reader: None,
                                writer: Some(f),
                                path,
                            },
                        );
                        Expr::Integer(handle)
                    }
                    Err(_) => Expr::Nil,
                }
            }
            _ => Expr::Nil,
        }
    }

    fn builtin_close(&mut self, args: &[Expr]) -> Expr {
        if let Some(handle) = args.first().and_then(|e| e.as_integer()) {
            self.file_handles.remove(&handle);
            Expr::Nil
        } else {
            Expr::Nil
        }
    }

    fn builtin_read_line(&mut self, args: &[Expr]) -> Expr {
        let handle = args.first().and_then(|e| e.as_integer()).unwrap_or(0);
        if let Some(fh) = self.file_handles.get_mut(&handle) {
            if let Some(reader) = &mut fh.reader {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => Expr::Nil, // EOF
                    Ok(_) => {
                        // Remove trailing newline (DOS style)
                        let trimmed = line.trim_end_matches(['\n', '\r']);
                        Expr::String(trimmed.to_string())
                    }
                    Err(_) => Expr::Nil,
                }
            } else {
                Expr::Nil
            }
        } else {
            Expr::Nil
        }
    }

    fn builtin_read_char(&mut self, args: &[Expr]) -> Expr {
        let handle = args.first().and_then(|e| e.as_integer()).unwrap_or(0);
        if let Some(fh) = self.file_handles.get_mut(&handle) {
            if let Some(reader) = &mut fh.reader {
                let mut buf = [0u8; 1];
                match std::io::Read::read(reader.get_mut(), &mut buf) {
                    Ok(1) => Expr::Integer(buf[0] as i32),
                    _ => Expr::Nil,
                }
            } else {
                Expr::Nil
            }
        } else {
            Expr::Nil
        }
    }

    fn builtin_write_line(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let text = match &args[0] {
            Expr::String(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let handle = args[1].as_integer().unwrap_or(0);
        if let Some(fh) = self.file_handles.get_mut(&handle) {
            if let Some(writer) = &mut fh.writer {
                let _ = writeln!(writer, "{}", text);
                return Expr::String(text);
            }
        }
        Expr::Nil
    }

    fn builtin_write_char(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let ch = args[0].as_integer().unwrap_or(0);
        let handle = args[1].as_integer().unwrap_or(0);
        if let Some(fh) = self.file_handles.get_mut(&handle) {
            if let Some(writer) = &mut fh.writer {
                let _ = writer.write_all(&[ch as u8]);
                return Expr::Integer(ch);
            }
        }
        Expr::Nil
    }

    fn builtin_findfile(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(path)) => {
                if std::path::Path::new(path).exists() {
                    Expr::String(path.clone())
                } else {
                    Expr::Nil
                }
            }
            _ => Expr::Nil,
        }
    }

    // AutoCAD specific

    fn builtin_command(&mut self, args: &[Expr]) -> Expr {
        // Log the command for simulation/output
        let cmd_parts: Vec<String> = args
            .iter()
            .map(|a| match a {
                Expr::String(s) => s.clone(),
                Expr::Integer(i) => i.to_string(),
                Expr::Real(r) => format!("{}", r),
                Expr::List(pt) if pt.len() >= 2 => {
                    let x = pt[0].as_real().unwrap_or(0.0);
                    let y = pt[1].as_real().unwrap_or(0.0);
                    format!("{},{}", x, y)
                }
                other => format!("{}", other),
            })
            .collect();

        let cmd_str = cmd_parts.join(" ");
        self.command_log.push(cmd_str.clone());

        // Simulate some basic commands
        if !args.is_empty() {
            if let Expr::String(cmd) = &args[0] {
                match cmd.to_uppercase().as_str() {
                    "LINE" => self.simulate_line_command(&args[1..]),
                    "CIRCLE" => self.simulate_circle_command(&args[1..]),
                    "TEXT" => self.simulate_text_command(&args[1..]),
                    "INSERT" => self.simulate_insert_command(&args[1..]),
                    "LAYER" => self.simulate_layer_command(&args[1..]),
                    "-LAYER" => self.simulate_layer_command(&args[1..]),
                    "ZOOM" => {} // View manipulation
                    "PAN" => {}
                    "SAVE" => {}
                    "SAVEAS" => {}
                    _ => {}
                }
            }
        }

        Expr::Nil
    }

    fn simulate_line_command(&mut self, args: &[Expr]) {
        // Very simplified LINE simulation
        let mut points: Vec<(f64, f64)> = Vec::new();
        for arg in args {
            if let Expr::List(pt) = arg {
                if pt.len() >= 2 {
                    let x = pt[0].as_real().unwrap_or(0.0);
                    let y = pt[1].as_real().unwrap_or(0.0);
                    points.push((x, y));
                }
            }
        }
        for i in 0..points.len().saturating_sub(1) {
            self.drawing.entities.push(DrawEntity::Line {
                x1: points[i].0,
                y1: points[i].1,
                x2: points[i + 1].0,
                y2: points[i + 1].1,
                layer: self.drawing.current_layer.clone(),
            });
        }
    }

    fn simulate_circle_command(&mut self, args: &[Expr]) {
        if args.len() >= 2 {
            if let (Expr::List(center), radius) = (&args[0], &args[1]) {
                if center.len() >= 2 {
                    let cx = center[0].as_real().unwrap_or(0.0);
                    let cy = center[1].as_real().unwrap_or(0.0);
                    let r = radius.as_real().unwrap_or(1.0);
                    self.drawing.entities.push(DrawEntity::Circle {
                        cx,
                        cy,
                        radius: r,
                        layer: self.drawing.current_layer.clone(),
                    });
                }
            }
        }
    }

    fn simulate_text_command(&mut self, args: &[Expr]) {
        // AutoCAD TEXT command: (point height rotation text)
        // Or simplified: (point height text)
        if args.len() >= 3 {
            if let Expr::List(pt) = &args[0] {
                if pt.len() >= 2 {
                    let x = pt[0].as_real().unwrap_or(0.0);
                    let y = pt[1].as_real().unwrap_or(0.0);
                    let height = args[1].as_real().unwrap_or(2.5);

                    // Check if we have 4 args (with rotation) or 3 args (without)
                    let text = if args.len() >= 4 {
                        // Format: point height rotation text
                        match &args[3] {
                            Expr::String(s) => s.clone(),
                            other => format!("{}", other),
                        }
                    } else {
                        // Format: point height text
                        match &args[2] {
                            Expr::String(s) => s.clone(),
                            other => format!("{}", other),
                        }
                    };

                    self.drawing.entities.push(DrawEntity::Text {
                        x,
                        y,
                        height,
                        text,
                        layer: self.drawing.current_layer.clone(),
                    });
                }
            }
        }
    }

    fn simulate_insert_command(&mut self, args: &[Expr]) {
        // INSERT blockname point scale rotation
        if !args.is_empty() {
            let block_name = match &args[0] {
                Expr::String(s) => s.clone(),
                _ => return,
            };
            let (x, y) = if args.len() > 1 {
                if let Expr::List(pt) = &args[1] {
                    (
                        pt.first().and_then(|e| e.as_real()).unwrap_or(0.0),
                        pt.get(1).and_then(|e| e.as_real()).unwrap_or(0.0),
                    )
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };
            let scale = args.get(2).and_then(|e| e.as_real()).unwrap_or(1.0);
            let rotation = args.get(3).and_then(|e| e.as_real()).unwrap_or(0.0);

            self.drawing.entities.push(DrawEntity::Insert {
                block_name,
                x,
                y,
                scale,
                rotation,
                layer: self.drawing.current_layer.clone(),
            });
        }
    }

    fn simulate_layer_command(&mut self, args: &[Expr]) {
        // Basic simulation: (command "LAYER" "S" "LayerName" "")
        // Args usually: "S" "LayerName" ""
        let mut i = 0;
        while i < args.len() {
            if let Expr::String(opt) = &args[i] {
                match opt.to_uppercase().as_str() {
                    "S" | "SET" => {
                        if i + 1 < args.len() {
                            if let Expr::String(layer) = &args[i + 1] {
                                self.drawing.current_layer = layer.clone();
                                self.drawing
                                    .system_variables
                                    .insert("CLAYER".to_string(), Expr::String(layer.clone()));
                            }
                            i += 1;
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
    }

    fn builtin_getvar(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(name)) => self
                .drawing
                .system_variables
                .get(&name.to_uppercase())
                .cloned()
                .unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_setvar(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        if let Expr::String(name) = &args[0] {
            self.drawing
                .system_variables
                .insert(name.to_uppercase(), args[1].clone());
            return args[1].clone();
        }
        Expr::Nil
    }

    // Geometry

    fn builtin_distance(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let (x1, y1) = match &args[0] {
            Expr::List(pt) if pt.len() >= 2 => (
                pt[0].as_real().unwrap_or(0.0),
                pt[1].as_real().unwrap_or(0.0),
            ),
            _ => return Expr::Nil,
        };
        let (x2, y2) = match &args[1] {
            Expr::List(pt) if pt.len() >= 2 => (
                pt[0].as_real().unwrap_or(0.0),
                pt[1].as_real().unwrap_or(0.0),
            ),
            _ => return Expr::Nil,
        };
        Expr::Real(((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt())
    }

    fn builtin_angle(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let (x1, y1) = match &args[0] {
            Expr::List(pt) if pt.len() >= 2 => (
                pt[0].as_real().unwrap_or(0.0),
                pt[1].as_real().unwrap_or(0.0),
            ),
            _ => return Expr::Nil,
        };
        let (x2, y2) = match &args[1] {
            Expr::List(pt) if pt.len() >= 2 => (
                pt[0].as_real().unwrap_or(0.0),
                pt[1].as_real().unwrap_or(0.0),
            ),
            _ => return Expr::Nil,
        };
        Expr::Real((y2 - y1).atan2(x2 - x1))
    }

    fn builtin_polar(&self, args: &[Expr]) -> Expr {
        if args.len() < 3 {
            return Expr::Nil;
        }
        let (x, y) = match &args[0] {
            Expr::List(pt) if pt.len() >= 2 => (
                pt[0].as_real().unwrap_or(0.0),
                pt[1].as_real().unwrap_or(0.0),
            ),
            _ => return Expr::Nil,
        };
        let angle = args[1].as_real().unwrap_or(0.0);
        let dist = args[2].as_real().unwrap_or(0.0);
        Expr::list(vec![
            Expr::Real(x + dist * angle.cos()),
            Expr::Real(y + dist * angle.sin()),
        ])
    }

    fn builtin_inters(&self, args: &[Expr]) -> Expr {
        // Line-line intersection
        if args.len() < 4 {
            return Expr::Nil;
        }
        // Simplified - just return nil for now
        Expr::Nil
    }

    fn builtin_entmake(&mut self, args: &[Expr]) -> Expr {
        // Create entity from DXF-like list
        if let Some(Expr::List(elist)) = args.first() {
            // Parse entity type from group 0
            let entity_type = elist.iter().find_map(|item| {
                if let Expr::List(pair) = item {
                    if pair.len() >= 2 {
                        if let (Expr::Integer(0), Expr::String(t)) = (&pair[0], &pair[1]) {
                            return Some(t.clone());
                        }
                    }
                }
                None
            });

            if let Some(etype) = entity_type {
                self.output.push(format!("ENTMAKE: {} created", etype));
                return Expr::List(elist.clone());
            }
        }
        Expr::Nil
    }

    // KiCad Extensions

    fn builtin_kicad_pin(&mut self, args: &[Expr]) -> Expr {
        // (kicad-pin name number type style x y length rotation)
        if args.len() < 8 {
            return Expr::Nil;
        }

        let name = match &args[0] {
            Expr::String(s) => s.clone(),
            other => format!("{}", other),
        };
        let number = match &args[1] {
            Expr::String(s) => s.clone(),
            other => format!("{}", other),
        };
        let etype = match &args[2] {
            Expr::String(s) => s.clone(),
            _ => "passive".to_string(),
        };
        let style = match &args[3] {
            Expr::String(s) => s.clone(),
            _ => "line".to_string(),
        };
        let x = args[4].as_real().unwrap_or(0.0);
        let y = args[5].as_real().unwrap_or(0.0);
        let length = args[6].as_real().unwrap_or(2.54); // Default 0.1"
        let rotation = args[7].as_real().unwrap_or(0.0);

        self.drawing.entities.push(DrawEntity::Pin {
            name: name.clone(),
            number,
            etype,
            style,
            x,
            y,
            length,
            rotation,
            layer: self.drawing.current_layer.clone(),
        });

        Expr::String(name)
    }

    fn builtin_kicad_prop(&mut self, args: &[Expr]) -> Expr {
        // (kicad-prop key value x y rotation height visible)
        if args.len() < 7 {
            return Expr::Nil;
        }

        let key = match &args[0] {
            Expr::String(s) => s.clone(),
            other => format!("{}", other),
        };
        let value = match &args[1] {
            Expr::String(s) => s.clone(),
            other => format!("{}", other),
        };
        let x = args[2].as_real().unwrap_or(0.0);
        let y = args[3].as_real().unwrap_or(0.0);
        let rotation = args[4].as_real().unwrap_or(0.0);
        let height = args[5].as_real().unwrap_or(1.27); // Default 0.05"
        let visible = args[6].is_truthy();

        self.drawing.entities.push(DrawEntity::Property {
            key: key.clone(),
            value,
            x,
            y,
            rotation,
            height,
            visible,
            layer: self.drawing.current_layer.clone(),
        });

        Expr::String(key)
    }

    fn builtin_kicad_pad(&mut self, args: &[Expr]) -> Expr {
        // (kicad-pad name type shape x y w h drill rot layers)
        if args.len() < 10 {
            return Expr::Nil;
        }

        let name = match &args[0] {
            Expr::String(s) => s.clone(),
            other => format!("{}", other),
        };
        let ptype = match &args[1] {
            Expr::String(s) => s.clone(),
            _ => "smd".to_string(),
        };
        let shape = match &args[2] {
            Expr::String(s) => s.clone(),
            _ => "rect".to_string(),
        };
        let x = args[3].as_real().unwrap_or(0.0);
        let y = args[4].as_real().unwrap_or(0.0);
        let width = args[5].as_real().unwrap_or(1.0);
        let height = args[6].as_real().unwrap_or(1.0);
        let drill = args[7].as_real().unwrap_or(0.0);
        let rotation = args[8].as_real().unwrap_or(0.0);
        let layers = match &args[9] {
            Expr::String(s) => s.clone(),
            _ => "F.Cu".to_string(),
        };

        self.drawing.entities.push(DrawEntity::Pad {
            name: name.clone(),
            ptype,
            shape,
            x,
            y,
            width,
            height,
            drill,
            rotation,
            layers,
        });

        Expr::String(name)
    }

    fn builtin_apply(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let func_name = match &args[0] {
            Expr::Quote(e) => match e.as_ref() {
                Expr::Symbol(s) => *s,
                _ => return Expr::Nil,
            },
            Expr::Symbol(s) => *s,
            _ => return Expr::Nil,
        };
        let arg_list = match &args[1] {
            Expr::List(v) => v.to_vec(),
            Expr::Nil => Vec::new(),
            _ => return Expr::Nil,
        };
        let mut call = vec![Expr::Symbol(func_name)];
        // arg_list holds values already: quote them so lists/symbols are not re-evaluated
        call.extend(arg_list.into_iter().map(|e| Expr::Quote(Box::new(e))));
        self.eval(&Expr::list(call))
    }

    fn builtin_load(&mut self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::String(path)) => match std::fs::read_to_string(path) {
                Ok(content) => {
                    let mut parser = crate::parser::Parser::from_input(&content);
                    let exprs = parser.parse_all();
                    let mut result = Expr::Nil;
                    for expr in exprs {
                        result = self.eval(&expr);
                    }
                    result
                }
                Err(e) => {
                    self.output.push(format!("Error loading {}: {}", path, e));
                    Expr::Nil
                }
            },
            _ => Expr::Nil,
        }
    }

    fn builtin_alert(&mut self, args: &[Expr]) -> Expr {
        if let Some(Expr::String(msg)) = args.first() {
            self.output.push(format!("[ALERT] {}", msg));
        }
        Expr::Nil
    }

    pub fn run(&mut self, code: &str) -> Vec<Expr> {
        let mut parser = crate::parser::Parser::from_input(code);
        let exprs = parser.parse_all();
        exprs.iter().map(|e| self.eval_top(e)).collect()
    }

    /// Evaluate a top-level form through the compiler (no frame, so no slots,
    /// but constants, control forms and builtin dispatch are resolved once;
    /// a `while` loop at top level runs its compiled body).
    pub fn eval_top(&mut self, expr: &Expr) -> Expr {
        let code = compile_body(&[], std::slice::from_ref(expr));
        code[0].run(self)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        let mut interp = Interpreter::new();
        let results = interp.run("(+ 1 2 3)");
        assert_eq!(results[0], Expr::Integer(6));
    }

    #[test]
    fn test_setq() {
        let mut interp = Interpreter::new();
        interp.run("(setq x 42)");
        let results = interp.run("x");
        assert_eq!(results[0], Expr::Integer(42));
    }

    #[test]
    fn test_defun() {
        let mut interp = Interpreter::new();
        interp.run("(defun square (x) (* x x))");
        let results = interp.run("(square 5)");
        assert_eq!(results[0], Expr::Integer(25));
    }

    #[test]
    fn test_list_ops() {
        let mut interp = Interpreter::new();
        let results = interp.run("(car '(1 2 3))");
        assert_eq!(results[0], Expr::Integer(1));
    }

    #[test]
    fn mapcar_and_apply_do_not_reevaluate_values() {
        let mut interp = Interpreter::new();
        let results = interp.run("(mapcar 'car '((1 2) (3 4)))");
        assert_eq!(results[0], Expr::list(vec![Expr::Integer(1), Expr::Integer(3)]));
        let results = interp.run("(apply 'length '((a b c)))");
        assert_eq!(results[0], Expr::Integer(3));
    }

    #[test]
    fn deep_recursion_is_reported_not_fatal() {
        let mut interp = Interpreter::new();
        interp.run("(defun r (n) (if (= n 0) 0 (+ 1 (r (- n 1))))) (r 100000)");
        assert!(interp
            .output
            .iter()
            .any(|l| l.contains("recursion too deep")));
        assert_eq!(interp.call_depth, 0, "depth counter must unwind");
    }

    #[test]
    fn cons_cells_are_autolisp_lists() {
        let mut interp = Interpreter::new();
        let r = interp.run(
            "(cons 1 2) (cons 1 '(2 3)) (cdr '(1 2 3)) (cddr '(1 2 3)) (cdr '(1)) \
             (append '(1 2) '(3) nil '(4 5)) (reverse '(1 2 3)) (member 2 '(1 2 3)) \
             (last '(1 2 3)) (nth 1 '(a b c)) (length nil) (car nil) (list) '()",
        );
        let shown: Vec<String> = r.iter().map(|e| e.to_string()).collect();
        assert_eq!(
            shown,
            vec![
                "(1 . 2)", "(1 2 3)", "(2 3)", "(3)", "nil", "(1 2 3 4 5)", "(3 2 1)", "(2 3)",
                "3", "B", "0", "nil", "nil", "nil"
            ]
        );
    }

    #[test]
    fn idiomatic_cdr_loop_and_sharing() {
        let mut interp = Interpreter::new();
        let r = interp.run(
            "(setq lst nil i 3000) \
             (while (> i 0) (setq lst (cons i lst) i (1- i))) \
             (setq n 0) \
             (while lst (setq n (+ n (car lst)) lst (cdr lst))) \
             n",
        );
        assert_eq!(r.last(), Some(&Expr::Integer(3000 * 3001 / 2)));
        // structural equality on shared and unshared lists
        let r = interp.run("(setq a '(1 2 3)) (setq b (cons 1 (cdr a))) (equal a b)");
        assert_eq!(r.last(), Some(&Expr::T));
    }

    #[test]
    fn setq_assigns_innermost_binding() {
        let mut interp = Interpreter::new();
        // a parameter shadows a global of the same name; setq inside the
        // function changes the parameter, not the global (dynamic scoping)
        let r = interp.run(
            "(setq x 1) (defun f (x) (setq x 5) x) (list (f 2) x) \
             (defun g (/ y) (setq y 7) y) (list (g) y)",
        );
        assert_eq!(r[2].to_string(), "(5 1)");
        assert_eq!(r[4].to_string(), "(7 nil)");
    }

    #[test]
    fn user_function_may_shadow_builtin() {
        let mut interp = Interpreter::new();
        let r = interp.run("(defun polar (p a d) 42) (polar '(0 0) 0 1)");
        assert_eq!(r.last(), Some(&Expr::Integer(42)));
    }
}

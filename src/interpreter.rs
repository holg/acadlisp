// AutoLISP Interpreter - AutoCAD 9/10 (DOS era) compatible
// Implements the subset of functions available in late 1980s AutoCAD

use crate::parser::Expr;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;

/// Context passed to foreign (extension) functions.
///
/// This avoids borrow conflicts with the Interpreter by only exposing
/// the mutable fields that extensions typically need.
pub struct ForeignContext<'a> {
    pub output: &'a mut Vec<String>,
    pub globals: &'a mut HashMap<String, Expr>,
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
#[derive(Debug, Clone)]
pub struct UserFunction {
    pub params: Vec<String>,
    pub body: Vec<Expr>,
}

pub struct Interpreter {
    // Global symbol table
    pub globals: HashMap<String, Expr>,
    // User-defined functions
    pub functions: HashMap<String, UserFunction>,
    // Local scope stack
    pub locals: Vec<HashMap<String, Expr>>,
    // Open file handles (AutoCAD 9/10 style - integer descriptors)
    pub file_handles: HashMap<i32, FileHandle>,
    next_handle: i32,
    // Simulated drawing state
    pub drawing: DrawingState,
    // Command queue for batch output
    pub command_log: Vec<String>,
    // Output buffer for PRINC/PRINT
    pub output: Vec<String>,
    // Foreign (extension) functions registered by embedding applications
    pub foreign_fns: HashMap<String, ForeignFn>,
    // Opaque user data for embedding applications
    pub user_data: Box<dyn std::any::Any + Send>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            globals: HashMap::new(),
            functions: HashMap::new(),
            locals: Vec::new(),
            file_handles: HashMap::new(),
            next_handle: 1,
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
        self.foreign_fns
            .insert(name.to_uppercase(), Arc::new(f));
    }

    fn get_var(&self, name: &str) -> Expr {
        // Check local scopes first (innermost to outermost)
        for scope in self.locals.iter().rev() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
        }
        // Then check globals
        self.globals.get(name).cloned().unwrap_or(Expr::Nil)
    }

    fn set_var(&mut self, name: &str, value: Expr) {
        // AutoLISP convention: variables starting with * are global
        // Also check if the variable already exists in globals
        let is_global = name.starts_with('*') || self.globals.contains_key(name);

        if is_global {
            self.globals.insert(name.to_string(), value);
        } else if let Some(scope) = self.locals.last_mut() {
            scope.insert(name.to_string(), value);
        } else {
            self.globals.insert(name.to_string(), value);
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Expr {
        match expr {
            Expr::Nil => Expr::Nil,
            Expr::T => Expr::T,
            Expr::Integer(i) => Expr::Integer(*i),
            Expr::Real(r) => Expr::Real(*r),
            Expr::String(s) => Expr::String(s.clone()),
            Expr::Symbol(name) => self.get_var(name),
            Expr::Quote(e) => (**e).clone(),
            Expr::List(items) if items.is_empty() => Expr::Nil,
            Expr::List(items) => self.eval_list(items),
        }
    }

    fn eval_list(&mut self, items: &[Expr]) -> Expr {
        if items.is_empty() {
            return Expr::Nil;
        }

        let first = &items[0];
        let func_name = match first {
            Expr::Symbol(name) => name.as_str(),
            _ => {
                // Could be a lambda or other callable - not in AutoCAD 9/10
                return Expr::Nil;
            }
        };

        let args = &items[1..];

        // Special forms (don't evaluate args)
        match func_name {
            "QUOTE" => return args.first().cloned().unwrap_or(Expr::Nil),
            "SETQ" => return self.builtin_setq(args),
            "DEFUN" => return self.builtin_defun(args),
            "IF" => return self.builtin_if(args),
            "COND" => return self.builtin_cond(args),
            "WHILE" => return self.builtin_while(args),
            "REPEAT" => return self.builtin_repeat(args),
            "PROGN" => return self.builtin_progn(args),
            "AND" => return self.builtin_and(args),
            "OR" => return self.builtin_or(args),
            "FOREACH" => return self.builtin_foreach(args),
            "LAMBDA" => return Expr::Nil, // Not really used in AutoCAD 9/10
            _ => {}
        }

        // Regular functions - evaluate args first
        let evaled_args: Vec<Expr> = args.iter().map(|a| self.eval(a)).collect();

        // Check for user-defined function
        if let Some(func) = self.functions.get(func_name).cloned() {
            return self.call_user_function(&func, &evaled_args);
        }

        // Check for foreign (extension) functions
        if let Some(f) = self.foreign_fns.get(func_name).cloned() {
            let mut ctx = ForeignContext {
                output: &mut self.output,
                globals: &mut self.globals,
                drawing: &mut self.drawing,
                user_data: self.user_data.as_mut(),
            };
            return f(&mut ctx, &evaled_args);
        }

        // Built-in functions
        match func_name {
            // Arithmetic
            "+" => self.builtin_add(&evaled_args),
            "-" => self.builtin_sub(&evaled_args),
            "*" => self.builtin_mul(&evaled_args),
            "/" => self.builtin_div(&evaled_args),
            "1+" => self.builtin_1plus(&evaled_args),
            "1-" => self.builtin_1minus(&evaled_args),
            "ABS" => self.builtin_abs(&evaled_args),
            "MAX" => self.builtin_max(&evaled_args),
            "MIN" => self.builtin_min(&evaled_args),
            "REM" => self.builtin_rem(&evaled_args),
            "GCD" => self.builtin_gcd(&evaled_args),
            "SIN" => self.builtin_sin(&evaled_args),
            "COS" => self.builtin_cos(&evaled_args),
            "ATAN" => self.builtin_atan(&evaled_args),
            "SQRT" => self.builtin_sqrt(&evaled_args),
            "EXPT" => self.builtin_expt(&evaled_args),
            "EXP" => self.builtin_exp(&evaled_args),
            "LOG" => self.builtin_log(&evaled_args),
            "FIX" => self.builtin_fix(&evaled_args),
            "FLOAT" => self.builtin_float(&evaled_args),

            // Comparison
            "=" => self.builtin_eq(&evaled_args),
            "/=" => self.builtin_neq(&evaled_args),
            "<" => self.builtin_lt(&evaled_args),
            ">" => self.builtin_gt(&evaled_args),
            "<=" => self.builtin_le(&evaled_args),
            ">=" => self.builtin_ge(&evaled_args),
            "EQ" => self.builtin_eq_sym(&evaled_args),
            "EQUAL" => self.builtin_equal(&evaled_args),

            // Logical
            "NOT" => self.builtin_not(&evaled_args),
            "NULL" => self.builtin_null(&evaled_args),

            // Type predicates
            "ATOM" => self.builtin_atom(&evaled_args),
            "LISTP" => self.builtin_listp(&evaled_args),
            "NUMBERP" => self.builtin_numberp(&evaled_args),
            "MINUSP" => self.builtin_minusp(&evaled_args),
            "ZEROP" => self.builtin_zerop(&evaled_args),
            "TYPE" => self.builtin_type(&evaled_args),

            // List operations
            "CAR" => self.builtin_car(&evaled_args),
            "CDR" => self.builtin_cdr(&evaled_args),
            "CADR" => self.builtin_cadr(&evaled_args),
            "CADDR" => self.builtin_caddr(&evaled_args),
            "CAAR" => self.builtin_caar(&evaled_args),
            "CADAR" => self.builtin_cadar(&evaled_args),
            "CDDR" => self.builtin_cddr(&evaled_args),
            "CONS" => self.builtin_cons(&evaled_args),
            "LIST" => self.builtin_list(&evaled_args),
            "APPEND" => self.builtin_append(&evaled_args),
            "REVERSE" => self.builtin_reverse(&evaled_args),
            "LENGTH" => self.builtin_length(&evaled_args),
            "NTH" => self.builtin_nth(&evaled_args),
            "LAST" => self.builtin_last(&evaled_args),
            "MEMBER" => self.builtin_member(&evaled_args),
            "ASSOC" => self.builtin_assoc(&evaled_args),
            "SUBST" => self.builtin_subst(&evaled_args),
            "MAPCAR" => self.builtin_mapcar(args), // Special - needs unevaled

            // String operations
            "STRCAT" => self.builtin_strcat(&evaled_args),
            "STRLEN" => self.builtin_strlen(&evaled_args),
            "SUBSTR" => self.builtin_substr(&evaled_args),
            "STRCASE" => self.builtin_strcase(&evaled_args),
            "ASCII" => self.builtin_ascii(&evaled_args),
            "CHR" => self.builtin_chr(&evaled_args),
            "ATOI" => self.builtin_atoi(&evaled_args),
            "ATOF" => self.builtin_atof(&evaled_args),
            "ITOA" => self.builtin_itoa(&evaled_args),
            "RTOS" => self.builtin_rtos(&evaled_args),
            "ANGTOS" => self.builtin_angtos(&evaled_args),
            "READ" => self.builtin_read_str(&evaled_args),

            // I/O
            "PRINC" => self.builtin_princ(&evaled_args),
            "PRINT" => self.builtin_print(&evaled_args),
            "PRIN1" => self.builtin_prin1(&evaled_args),
            "PROMPT" => self.builtin_prompt(&evaled_args),
            "TERPRI" => self.builtin_terpri(&evaled_args),

            // File I/O (DOS style)
            "OPEN" => self.builtin_open(&evaled_args),
            "CLOSE" => self.builtin_close(&evaled_args),
            "READ-LINE" => self.builtin_read_line(&evaled_args),
            "READ-CHAR" => self.builtin_read_char(&evaled_args),
            "WRITE-LINE" => self.builtin_write_line(&evaled_args),
            "WRITE-CHAR" => self.builtin_write_char(&evaled_args),
            "FINDFILE" => self.builtin_findfile(&evaled_args),

            // User input (simulated - returns nil in batch mode)
            "GETINT" => Expr::Nil,
            "GETREAL" => Expr::Nil,
            "GETSTRING" => Expr::Nil,
            "GETPOINT" => Expr::Nil,
            "GETCORNER" => Expr::Nil,
            "GETDIST" => Expr::Nil,
            "GETANGLE" => Expr::Nil,
            "GETKWORD" => Expr::Nil,
            "INITGET" => Expr::Nil,

            // AutoCAD specific
            "COMMAND" => self.builtin_command(&evaled_args),
            "GETVAR" => self.builtin_getvar(&evaled_args),
            "SETVAR" => self.builtin_setvar(&evaled_args),

            // Geometry (points as lists)
            "DISTANCE" => self.builtin_distance(&evaled_args),
            "ANGLE" => self.builtin_angle(&evaled_args),
            "POLAR" => self.builtin_polar(&evaled_args),
            "INTERS" => self.builtin_inters(&evaled_args),

            // Entity access (simulated)
            "ENTSEL" => Expr::Nil,
            "ENTGET" => Expr::Nil,
            "ENTMOD" => Expr::Nil,
            "ENTMAKE" => self.builtin_entmake(&evaled_args),
            "ENTNEXT" => Expr::Nil,
            "ENTLAST" => Expr::Nil,

            // KiCad extensions
            "KICAD-PIN" => self.builtin_kicad_pin(&evaled_args),
            "KICAD-PROP" => self.builtin_kicad_prop(&evaled_args),
            "KICAD-PAD" => self.builtin_kicad_pad(&evaled_args),

            // Selection sets (simulated)
            "SSGET" => Expr::Nil,
            "SSLENGTH" => Expr::Integer(0),
            "SSNAME" => Expr::Nil,
            "SSADD" => Expr::Nil,
            "SSDEL" => Expr::Nil,

            // Misc
            "EVAL" => {
                if let Some(e) = evaled_args.first() {
                    self.eval(e)
                } else {
                    Expr::Nil
                }
            }
            "APPLY" => self.builtin_apply(&evaled_args),
            "LOAD" => self.builtin_load(&evaled_args),
            "ALERT" => self.builtin_alert(&evaled_args),
            "QUIT" => {
                self.output.push("*** QUIT ***".to_string());
                Expr::Nil
            }
            "EXIT" => {
                self.output.push("*** EXIT ***".to_string());
                Expr::Nil
            }

            _ => {
                self.output
                    .push(format!("Error: unknown function {}", func_name));
                Expr::Nil
            }
        }
    }

    fn call_user_function(&mut self, func: &UserFunction, args: &[Expr]) -> Expr {
        // Create new local scope
        let mut scope = HashMap::new();
        for (i, param) in func.params.iter().enumerate() {
            let val = args.get(i).cloned().unwrap_or(Expr::Nil);
            scope.insert(param.clone(), val);
        }
        self.locals.push(scope);

        // Evaluate body
        let mut result = Expr::Nil;
        for expr in &func.body {
            result = self.eval(expr);
        }

        // Pop scope
        self.locals.pop();
        result
    }

    // Special forms

    fn builtin_setq(&mut self, args: &[Expr]) -> Expr {
        let mut result = Expr::Nil;
        let mut i = 0;
        while i + 1 < args.len() {
            if let Expr::Symbol(name) = &args[i] {
                let val = self.eval(&args[i + 1]);
                self.set_var(name, val.clone());
                result = val;
            }
            i += 2;
        }
        result
    }

    fn builtin_defun(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 3 {
            return Expr::Nil;
        }
        let name = match &args[0] {
            Expr::Symbol(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let params = match &args[1] {
            Expr::List(items) => {
                items
                    .iter()
                    .filter_map(|e| {
                        if let Expr::Symbol(s) = e {
                            // Skip / which separates required from local vars in AutoLISP
                            if s != "/" {
                                Some(s.clone())
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
        let body = args[2..].to_vec();
        self.functions
            .insert(name.clone(), UserFunction { params, body });
        Expr::Symbol(name)
    }

    fn builtin_if(&mut self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Nil;
        }
        let cond = self.eval(&args[0]);
        if cond.is_truthy() {
            args.get(1).map(|e| self.eval(e)).unwrap_or(Expr::Nil)
        } else {
            args.get(2).map(|e| self.eval(e)).unwrap_or(Expr::Nil)
        }
    }

    fn builtin_cond(&mut self, args: &[Expr]) -> Expr {
        for clause in args {
            if let Expr::List(items) = clause {
                if items.is_empty() {
                    continue;
                }
                let test = self.eval(&items[0]);
                if test.is_truthy() {
                    let mut result = test;
                    for expr in &items[1..] {
                        result = self.eval(expr);
                    }
                    return result;
                }
            }
        }
        Expr::Nil
    }

    fn builtin_while(&mut self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Nil;
        }
        let test = &args[0];
        let body = &args[1..];
        let mut result = Expr::Nil;
        while self.eval(test).is_truthy() {
            for expr in body {
                result = self.eval(expr);
            }
        }
        result
    }

    fn builtin_repeat(&mut self, args: &[Expr]) -> Expr {
        if args.is_empty() {
            return Expr::Nil;
        }
        let count = self.eval(&args[0]).as_integer().unwrap_or(0);
        let body = &args[1..];
        let mut result = Expr::Nil;
        for _ in 0..count {
            for expr in body {
                result = self.eval(expr);
            }
        }
        result
    }

    fn builtin_progn(&mut self, args: &[Expr]) -> Expr {
        let mut result = Expr::Nil;
        for expr in args {
            result = self.eval(expr);
        }
        result
    }

    fn builtin_and(&mut self, args: &[Expr]) -> Expr {
        for arg in args {
            let val = self.eval(arg);
            if !val.is_truthy() {
                return Expr::Nil;
            }
        }
        Expr::T
    }

    fn builtin_or(&mut self, args: &[Expr]) -> Expr {
        for arg in args {
            let val = self.eval(arg);
            if val.is_truthy() {
                return val;
            }
        }
        Expr::Nil
    }

    fn builtin_foreach(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 3 {
            return Expr::Nil;
        }
        let var_name = match &args[0] {
            Expr::Symbol(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let list = self.eval(&args[1]);
        let items = match list {
            Expr::List(v) => v,
            _ => return Expr::Nil,
        };
        let body = &args[2..];

        let mut result = Expr::Nil;
        for item in items {
            self.set_var(&var_name, item);
            for expr in body {
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
            Some(Expr::List(v)) if !v.is_empty() => Expr::Nil,
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
            Some(Expr::Integer(_)) => Expr::Symbol("INT".to_string()),
            Some(Expr::Real(_)) => Expr::Symbol("REAL".to_string()),
            Some(Expr::String(_)) => Expr::Symbol("STR".to_string()),
            Some(Expr::Symbol(_)) => Expr::Symbol("SYM".to_string()),
            Some(Expr::List(_)) => Expr::Symbol("LIST".to_string()),
            Some(Expr::Nil) | None => Expr::Nil,
            Some(Expr::T) => Expr::Symbol("SYM".to_string()),
            Some(Expr::Quote(_)) => Expr::Symbol("LIST".to_string()),
        }
    }

    // List operations

    fn builtin_car(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => v.first().cloned().unwrap_or(Expr::Nil),
            _ => Expr::Nil,
        }
    }

    fn builtin_cdr(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) if v.len() > 1 => Expr::List(v[1..].to_vec()),
            Some(Expr::List(_)) => Expr::Nil,
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
            Some(Expr::List(v)) if v.len() > 2 => Expr::List(v[2..].to_vec()),
            _ => Expr::Nil,
        }
    }

    fn builtin_cons(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let first = args[0].clone();
        match &args[1] {
            Expr::List(v) => {
                let mut new_list = vec![first];
                new_list.extend(v.iter().cloned());
                Expr::List(new_list)
            }
            Expr::Nil => Expr::List(vec![first]),
            other => {
                // Dotted pair - represent as 2-element list
                Expr::List(vec![first, other.clone()])
            }
        }
    }

    fn builtin_list(&self, args: &[Expr]) -> Expr {
        Expr::List(args.to_vec())
    }

    fn builtin_append(&self, args: &[Expr]) -> Expr {
        let mut result = Vec::new();
        for arg in args {
            if let Expr::List(v) = arg {
                result.extend(v.iter().cloned());
            }
        }
        Expr::List(result)
    }

    fn builtin_reverse(&self, args: &[Expr]) -> Expr {
        match args.first() {
            Some(Expr::List(v)) => {
                let mut rev = v.clone();
                rev.reverse();
                Expr::List(rev)
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
        if let Expr::List(v) = &args[1] {
            for (i, elem) in v.iter().enumerate() {
                if elem == item {
                    return Expr::List(v[i..].to_vec());
                }
            }
        }
        Expr::Nil
    }

    fn builtin_assoc(&self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let key = &args[0];
        if let Expr::List(alist) = &args[1] {
            for item in alist {
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
                Expr::List(result)
            }
            other => other.clone(),
        }
    }

    fn builtin_mapcar(&mut self, args: &[Expr]) -> Expr {
        if args.len() < 2 {
            return Expr::Nil;
        }
        let func_name = match &args[0] {
            Expr::Quote(e) => match e.as_ref() {
                Expr::Symbol(s) => s.clone(),
                _ => return Expr::Nil,
            },
            Expr::Symbol(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let list = self.eval(&args[1]);
        if let Expr::List(items) = list {
            let results: Vec<Expr> = items
                .iter()
                .map(|item| {
                    let call = Expr::List(vec![Expr::Symbol(func_name.clone()), item.clone()]);
                    self.eval(&call)
                })
                .collect();
            Expr::List(results)
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
        Expr::List(vec![
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
                Expr::Symbol(s) => s.clone(),
                _ => return Expr::Nil,
            },
            Expr::Symbol(s) => s.clone(),
            _ => return Expr::Nil,
        };
        let arg_list = match &args[1] {
            Expr::List(v) => v.clone(),
            _ => return Expr::Nil,
        };
        let mut call = vec![Expr::Symbol(func_name)];
        call.extend(arg_list);
        self.eval(&Expr::List(call))
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
        exprs.iter().map(|e| self.eval(e)).collect()
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
}

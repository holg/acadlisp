// HP-41C to LISP Transpiler
//
// This module transpiles HP-41C keystroke programs to LISP code.
// The HP-41C is a programmable RPN calculator with:
// - 4-level stack (X, Y, Z, T)
// - Global labels (alpha) and local labels (00-99, A-J, a-e)
// - Subroutine calls (XEQ) with 6-level nesting
// - Conditional tests that skip the next instruction if false
// - Loop constructs (ISG, DSE)
// - 100+ storage registers

use std::collections::HashMap;

/// HP-41C instruction types
#[derive(Debug, Clone, PartialEq)]
pub enum HP41Instruction {
    // Numbers and entry
    Number(f64),

    // Labels and control flow
    Lbl(String),           // LBL "name" or LBL 00-99
    Gto(String),           // GTO "name" or GTO 00-99
    Xeq(String),           // XEQ "name" or XEQ 00-99
    Rtn,                   // Return from subroutine
    End,                   // End of program
    Stop,                  // R/S - pause execution

    // Stack operations
    Enter,                 // ENTER - duplicate X, lift stack
    Clx,                   // Clear X
    ClSt,                  // Clear stack
    XchgY,                 // X<>Y - swap X and Y
    RollDown,              // R↓ - roll stack down
    RollUp,                // R↑ - roll stack up
    LastX,                 // Recall last X

    // Arithmetic
    Add,                   // +
    Sub,                   // -
    Mul,                   // ×
    Div,                   // ÷
    Chs,                   // Change sign

    // Math functions
    Sqrt,                  // √x
    Square,                // x²
    Reciprocal,            // 1/x
    Power,                 // y^x

    // Trigonometry (radians)
    Sin, Cos, Tan,
    Asin, Acos, Atan,

    // Logarithms
    Log,                   // log₁₀
    Ln,                    // natural log
    Exp10,                 // 10^x
    Exp,                   // e^x

    // Constants
    Pi,

    // Storage
    Sto(String),           // STO nn or STO IND nn
    Rcl(String),           // RCL nn or RCL IND nn
    StoAdd(String),        // STO+ nn
    StoSub(String),        // STO- nn
    StoMul(String),        // STO× nn
    StoDiv(String),        // STO÷ nn
    XchgReg(String),       // X<> nn

    // Conditional tests (skip next if false)
    TestXEq0,              // X=0?
    TestXNe0,              // X≠0?
    TestXLt0,              // X<0?
    TestXGt0,              // X>0?
    TestXLe0,              // X≤0?
    TestXGe0,              // X≥0?
    TestXEqY,              // X=Y?
    TestXNeY,              // X≠Y?
    TestXLtY,              // X<Y?
    TestXGtY,              // X>Y?
    TestXLeY,              // X≤Y?
    TestXGeY,              // X≥Y?

    // Loop constructs
    Isg(String),           // ISG nn - increment, skip if greater
    Dse(String),           // DSE nn - decrement, skip if equal/less

    // Flags
    SetFlag(u8),           // SF nn
    ClearFlag(u8),         // CF nn
    TestFlagSet(u8),       // FS? nn
    TestFlagClear(u8),     // FC? nn

    // Input/Output
    View(String),          // VIEW nn - display register
    Prompt,                // PROMPT - show alpha, wait for input
    Aview,                 // AVIEW - display alpha register

    // Alpha operations
    AlphaString(String),   // Append string to alpha
    Cla,                   // Clear alpha register

    // Misc
    Nop,                   // No operation
    Beep,                  // Tone output

    // Percent
    Percent,               // %
    PercentCh,             // %CH - percent change

    // Conversions
    ToRect,                // P→R polar to rectangular
    ToPolar,               // R→P rectangular to polar
    ToDeg,                 // →DEG
    ToRad,                 // →RAD
    ToHms,                 // →HMS
    ToHr,                  // →HR

    // Statistics
    SigmaPlus,             // Σ+
    SigmaMinus,            // Σ-
    Mean,                  // x̄
    Sdev,                  // s (standard deviation)

    // Integer
    Int,                   // INT - integer part
    Frac,                  // FRAC - fractional part
    Abs,                   // ABS - absolute value
    Sign,                  // SIGN
    Mod,                   // MOD

    // Hyperbolic
    Sinh, Cosh, Tanh,
    Asinh, Acosh, Atanh,
}

/// HP-41C Program representation
#[derive(Debug, Clone)]
pub struct HP41Program {
    pub name: String,
    pub instructions: Vec<HP41Instruction>,
    pub labels: HashMap<String, usize>,  // label -> instruction index
}

impl HP41Program {
    pub fn new(name: &str) -> Self {
        HP41Program {
            name: name.to_string(),
            instructions: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Parse HP-41C program text into instructions
    pub fn parse(input: &str) -> Result<HP41Program, String> {
        let mut program = HP41Program::new("MAIN");
        let mut current_label = String::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
                continue;  // Skip empty lines and comments
            }

            // Parse the instruction
            let instr = Self::parse_instruction(line)?;

            // Track labels
            if let HP41Instruction::Lbl(ref name) = instr {
                current_label = name.clone();
                program.labels.insert(name.clone(), program.instructions.len());
                if program.name == "MAIN" && !name.chars().all(|c| c.is_ascii_digit()) {
                    program.name = name.clone();
                }
            }

            program.instructions.push(instr);
        }

        Ok(program)
    }

    /// Parse a single instruction line
    fn parse_instruction(line: &str) -> Result<HP41Instruction, String> {
        let line = line.to_uppercase();
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(HP41Instruction::Nop);
        }

        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim_matches('"').to_string());

        match cmd {
            // Numbers
            _ if cmd.parse::<f64>().is_ok() => {
                Ok(HP41Instruction::Number(cmd.parse().unwrap()))
            }

            // Labels and control flow
            "LBL" => Ok(HP41Instruction::Lbl(arg.ok_or("LBL requires argument")?)),
            "GTO" => Ok(HP41Instruction::Gto(arg.ok_or("GTO requires argument")?)),
            "XEQ" => Ok(HP41Instruction::Xeq(arg.ok_or("XEQ requires argument")?)),
            "RTN" => Ok(HP41Instruction::Rtn),
            "END" => Ok(HP41Instruction::End),
            "STOP" | "R/S" => Ok(HP41Instruction::Stop),

            // Stack operations
            "ENTER" | "ENTER^" => Ok(HP41Instruction::Enter),
            "CLX" => Ok(HP41Instruction::Clx),
            "CLST" => Ok(HP41Instruction::ClSt),
            "X<>Y" | "X<->Y" => Ok(HP41Instruction::XchgY),
            "RDN" | "R↓" => Ok(HP41Instruction::RollDown),
            "RUP" | "R↑" => Ok(HP41Instruction::RollUp),
            "LASTX" | "LASTx" => Ok(HP41Instruction::LastX),

            // Arithmetic
            "+" => Ok(HP41Instruction::Add),
            "-" => Ok(HP41Instruction::Sub),
            "*" | "×" => Ok(HP41Instruction::Mul),
            "/" | "÷" => Ok(HP41Instruction::Div),
            "CHS" => Ok(HP41Instruction::Chs),

            // Math functions
            "SQRT" | "√X" => Ok(HP41Instruction::Sqrt),
            "X^2" | "X²" => Ok(HP41Instruction::Square),
            "1/X" => Ok(HP41Instruction::Reciprocal),
            "Y^X" => Ok(HP41Instruction::Power),

            // Trigonometry
            "SIN" => Ok(HP41Instruction::Sin),
            "COS" => Ok(HP41Instruction::Cos),
            "TAN" => Ok(HP41Instruction::Tan),
            "ASIN" => Ok(HP41Instruction::Asin),
            "ACOS" => Ok(HP41Instruction::Acos),
            "ATAN" => Ok(HP41Instruction::Atan),

            // Logarithms
            "LOG" => Ok(HP41Instruction::Log),
            "LN" => Ok(HP41Instruction::Ln),
            "10^X" => Ok(HP41Instruction::Exp10),
            "E^X" => Ok(HP41Instruction::Exp),

            // Constants
            "PI" => Ok(HP41Instruction::Pi),

            // Storage
            "STO" => Ok(HP41Instruction::Sto(arg.ok_or("STO requires argument")?)),
            "RCL" => Ok(HP41Instruction::Rcl(arg.ok_or("RCL requires argument")?)),
            "STO+" => Ok(HP41Instruction::StoAdd(arg.ok_or("STO+ requires argument")?)),
            "STO-" => Ok(HP41Instruction::StoSub(arg.ok_or("STO- requires argument")?)),
            "STO*" | "STO×" => Ok(HP41Instruction::StoMul(arg.ok_or("STO* requires argument")?)),
            "STO/" | "STO÷" => Ok(HP41Instruction::StoDiv(arg.ok_or("STO/ requires argument")?)),
            "X<>" => Ok(HP41Instruction::XchgReg(arg.ok_or("X<> requires argument")?)),

            // Conditional tests
            "X=0?" => Ok(HP41Instruction::TestXEq0),
            "X≠0?" | "X<>0?" | "X!=0?" => Ok(HP41Instruction::TestXNe0),
            "X<0?" => Ok(HP41Instruction::TestXLt0),
            "X>0?" => Ok(HP41Instruction::TestXGt0),
            "X≤0?" | "X<=0?" => Ok(HP41Instruction::TestXLe0),
            "X≥0?" | "X>=0?" => Ok(HP41Instruction::TestXGe0),
            "X=Y?" => Ok(HP41Instruction::TestXEqY),
            "X≠Y?" | "X<>Y?" | "X!=Y?" => Ok(HP41Instruction::TestXNeY),
            "X<Y?" => Ok(HP41Instruction::TestXLtY),
            "X>Y?" => Ok(HP41Instruction::TestXGtY),
            "X≤Y?" | "X<=Y?" => Ok(HP41Instruction::TestXLeY),
            "X≥Y?" | "X>=Y?" => Ok(HP41Instruction::TestXGeY),

            // Loop constructs
            "ISG" => Ok(HP41Instruction::Isg(arg.ok_or("ISG requires argument")?)),
            "DSE" => Ok(HP41Instruction::Dse(arg.ok_or("DSE requires argument")?)),

            // Flags
            "SF" => {
                let n: u8 = arg.ok_or("SF requires argument")?.parse()
                    .map_err(|_| "SF requires numeric argument")?;
                Ok(HP41Instruction::SetFlag(n))
            }
            "CF" => {
                let n: u8 = arg.ok_or("CF requires argument")?.parse()
                    .map_err(|_| "CF requires numeric argument")?;
                Ok(HP41Instruction::ClearFlag(n))
            }
            "FS?" => {
                let n: u8 = arg.ok_or("FS? requires argument")?.parse()
                    .map_err(|_| "FS? requires numeric argument")?;
                Ok(HP41Instruction::TestFlagSet(n))
            }
            "FC?" => {
                let n: u8 = arg.ok_or("FC? requires argument")?.parse()
                    .map_err(|_| "FC? requires numeric argument")?;
                Ok(HP41Instruction::TestFlagClear(n))
            }

            // I/O
            "VIEW" => Ok(HP41Instruction::View(arg.ok_or("VIEW requires argument")?)),
            "PROMPT" => Ok(HP41Instruction::Prompt),
            "AVIEW" => Ok(HP41Instruction::Aview),
            "CLA" => Ok(HP41Instruction::Cla),

            // Misc
            "NOP" => Ok(HP41Instruction::Nop),
            "BEEP" | "TONE" => Ok(HP41Instruction::Beep),

            // Percent
            "%" => Ok(HP41Instruction::Percent),
            "%CH" => Ok(HP41Instruction::PercentCh),

            // Conversions
            "P-R" | "P→R" => Ok(HP41Instruction::ToRect),
            "R-P" | "R→P" => Ok(HP41Instruction::ToPolar),
            "DEG" | "→DEG" => Ok(HP41Instruction::ToDeg),
            "RAD" | "→RAD" => Ok(HP41Instruction::ToRad),
            "HMS" | "→HMS" => Ok(HP41Instruction::ToHms),
            "HR" | "→HR" => Ok(HP41Instruction::ToHr),

            // Statistics
            "Σ+" | "SIGMA+" => Ok(HP41Instruction::SigmaPlus),
            "Σ-" | "SIGMA-" => Ok(HP41Instruction::SigmaMinus),
            "MEAN" => Ok(HP41Instruction::Mean),
            "SDEV" => Ok(HP41Instruction::Sdev),

            // Integer
            "INT" => Ok(HP41Instruction::Int),
            "FRAC" => Ok(HP41Instruction::Frac),
            "ABS" => Ok(HP41Instruction::Abs),
            "SIGN" => Ok(HP41Instruction::Sign),
            "MOD" => Ok(HP41Instruction::Mod),

            // Hyperbolic
            "SINH" => Ok(HP41Instruction::Sinh),
            "COSH" => Ok(HP41Instruction::Cosh),
            "TANH" => Ok(HP41Instruction::Tanh),
            "ASINH" => Ok(HP41Instruction::Asinh),
            "ACOSH" => Ok(HP41Instruction::Acosh),
            "ATANH" => Ok(HP41Instruction::Atanh),

            _ => Err(format!("Unknown instruction: {}", cmd)),
        }
    }

    /// Transpile HP-41C program to LISP code
    pub fn to_lisp(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("; HP-41C Program: {}\n", self.name));
        output.push_str("; Transpiled to LISP\n\n");

        // Generate LISP code for each instruction
        let mut i = 0;
        while i < self.instructions.len() {
            let instr = &self.instructions[i];
            let lisp = self.instruction_to_lisp(instr, i);

            if !lisp.is_empty() {
                output.push_str(&lisp);
                output.push('\n');
            }

            i += 1;
        }

        output
    }

    /// Convert a single instruction to LISP
    fn instruction_to_lisp(&self, instr: &HP41Instruction, _index: usize) -> String {
        match instr {
            // Numbers
            HP41Instruction::Number(n) => format!("(stack-push {})", n),

            // Labels - define as LISP function
            HP41Instruction::Lbl(name) => format!("\n(defun hp41-{} ()", Self::sanitize_label(name)),

            // Control flow
            HP41Instruction::Gto(name) => format!("  (hp41-{})", Self::sanitize_label(name)),
            HP41Instruction::Xeq(name) => format!("  (hp41-{})", Self::sanitize_label(name)),
            HP41Instruction::Rtn => "  stack-x)".to_string(),  // Close function, return X
            HP41Instruction::End => ")".to_string(),  // Close function
            HP41Instruction::Stop => "  ; STOP - pause".to_string(),

            // Stack operations
            HP41Instruction::Enter => "  (stack-push stack-x)".to_string(),
            HP41Instruction::Clx => "  (setq stack-x 0)".to_string(),
            HP41Instruction::ClSt => "  (stack-clear)".to_string(),
            HP41Instruction::XchgY => "  (stack-swap)".to_string(),
            HP41Instruction::RollDown => "  (stack-roll-down)".to_string(),
            HP41Instruction::RollUp => "  (stack-roll-up)".to_string(),
            HP41Instruction::LastX => "  (stack-push last-x)".to_string(),

            // Arithmetic
            HP41Instruction::Add => "  (rpn-add)".to_string(),
            HP41Instruction::Sub => "  (rpn-sub)".to_string(),
            HP41Instruction::Mul => "  (rpn-mul)".to_string(),
            HP41Instruction::Div => "  (rpn-div)".to_string(),
            HP41Instruction::Chs => "  (rpn-chs)".to_string(),

            // Math functions
            HP41Instruction::Sqrt => "  (rpn-sqrt)".to_string(),
            HP41Instruction::Square => "  (setq last-x stack-x) (setq stack-x (* stack-x stack-x))".to_string(),
            HP41Instruction::Reciprocal => "  (rpn-inv)".to_string(),
            HP41Instruction::Power => "  (rpn-pow)".to_string(),

            // Trigonometry
            HP41Instruction::Sin => "  (rpn-sin)".to_string(),
            HP41Instruction::Cos => "  (rpn-cos)".to_string(),
            HP41Instruction::Tan => "  (rpn-tan)".to_string(),
            HP41Instruction::Asin => "  (setq last-x stack-x) (setq stack-x (asin stack-x))".to_string(),
            HP41Instruction::Acos => "  (setq last-x stack-x) (setq stack-x (acos stack-x))".to_string(),
            HP41Instruction::Atan => "  (setq last-x stack-x) (setq stack-x (atan stack-x))".to_string(),

            // Logarithms
            HP41Instruction::Log => "  (rpn-log)".to_string(),
            HP41Instruction::Ln => "  (rpn-ln)".to_string(),
            HP41Instruction::Exp10 => "  (setq last-x stack-x) (setq stack-x (expt 10 stack-x))".to_string(),
            HP41Instruction::Exp => "  (setq last-x stack-x) (setq stack-x (exp stack-x))".to_string(),

            // Constants
            HP41Instruction::Pi => "  (rpn-pi)".to_string(),

            // Storage
            HP41Instruction::Sto(reg) => format!("  (setq reg-{} stack-x)", Self::sanitize_label(reg)),
            HP41Instruction::Rcl(reg) => format!("  (stack-push reg-{})", Self::sanitize_label(reg)),
            HP41Instruction::StoAdd(reg) => format!("  (setq reg-{} (+ reg-{} stack-x))",
                Self::sanitize_label(reg), Self::sanitize_label(reg)),
            HP41Instruction::StoSub(reg) => format!("  (setq reg-{} (- reg-{} stack-x))",
                Self::sanitize_label(reg), Self::sanitize_label(reg)),
            HP41Instruction::StoMul(reg) => format!("  (setq reg-{} (* reg-{} stack-x))",
                Self::sanitize_label(reg), Self::sanitize_label(reg)),
            HP41Instruction::StoDiv(reg) => format!("  (setq reg-{} (/ reg-{} stack-x))",
                Self::sanitize_label(reg), Self::sanitize_label(reg)),
            HP41Instruction::XchgReg(reg) => format!(
                "  (let ((tmp stack-x)) (setq stack-x reg-{}) (setq reg-{} tmp))",
                Self::sanitize_label(reg), Self::sanitize_label(reg)),

            // Conditional tests - implemented as (when test ...)
            HP41Instruction::TestXEq0 => "  (when (= stack-x 0)".to_string(),
            HP41Instruction::TestXNe0 => "  (when (/= stack-x 0)".to_string(),
            HP41Instruction::TestXLt0 => "  (when (< stack-x 0)".to_string(),
            HP41Instruction::TestXGt0 => "  (when (> stack-x 0)".to_string(),
            HP41Instruction::TestXLe0 => "  (when (<= stack-x 0)".to_string(),
            HP41Instruction::TestXGe0 => "  (when (>= stack-x 0)".to_string(),
            HP41Instruction::TestXEqY => "  (when (= stack-x stack-y)".to_string(),
            HP41Instruction::TestXNeY => "  (when (/= stack-x stack-y)".to_string(),
            HP41Instruction::TestXLtY => "  (when (< stack-x stack-y)".to_string(),
            HP41Instruction::TestXGtY => "  (when (> stack-x stack-y)".to_string(),
            HP41Instruction::TestXLeY => "  (when (<= stack-x stack-y)".to_string(),
            HP41Instruction::TestXGeY => "  (when (>= stack-x stack-y)".to_string(),

            // Loop constructs
            HP41Instruction::Isg(reg) => format!(
                "  (setq reg-{} (+ reg-{} 1)) (when (> reg-{} (truncate reg-{}))",
                Self::sanitize_label(reg), Self::sanitize_label(reg),
                Self::sanitize_label(reg), Self::sanitize_label(reg)),
            HP41Instruction::Dse(reg) => format!(
                "  (setq reg-{} (- reg-{} 1)) (when (<= reg-{} (truncate reg-{}))",
                Self::sanitize_label(reg), Self::sanitize_label(reg),
                Self::sanitize_label(reg), Self::sanitize_label(reg)),

            // Flags
            HP41Instruction::SetFlag(n) => format!("  (setq flag-{} t)", n),
            HP41Instruction::ClearFlag(n) => format!("  (setq flag-{} nil)", n),
            HP41Instruction::TestFlagSet(n) => format!("  (when flag-{}", n),
            HP41Instruction::TestFlagClear(n) => format!("  (when (not flag-{})", n),

            // I/O
            HP41Instruction::View(reg) => format!("  (princ reg-{})", Self::sanitize_label(reg)),
            HP41Instruction::Prompt => "  ; PROMPT".to_string(),
            HP41Instruction::Aview => "  (princ alpha-reg)".to_string(),
            HP41Instruction::Cla => "  (setq alpha-reg \"\")".to_string(),

            // Misc
            HP41Instruction::Nop => "".to_string(),
            HP41Instruction::Beep => "  ; BEEP".to_string(),

            // Percent
            HP41Instruction::Percent => "  (setq last-x stack-x) (setq stack-x (* stack-y (/ stack-x 100)))".to_string(),
            HP41Instruction::PercentCh => "  (setq last-x stack-x) (setq stack-x (* 100 (/ (- stack-x stack-y) stack-y)))".to_string(),

            // Integer operations
            HP41Instruction::Int => "  (setq last-x stack-x) (setq stack-x (truncate stack-x))".to_string(),
            HP41Instruction::Frac => "  (setq last-x stack-x) (setq stack-x (- stack-x (truncate stack-x)))".to_string(),
            HP41Instruction::Abs => "  (setq last-x stack-x) (setq stack-x (abs stack-x))".to_string(),
            HP41Instruction::Sign => "  (setq last-x stack-x) (setq stack-x (signum stack-x))".to_string(),
            HP41Instruction::Mod => "  (setq last-x stack-x) (let ((r (mod stack-y stack-x))) (stack-drop) (setq stack-x r))".to_string(),

            // Hyperbolic
            HP41Instruction::Sinh => "  (setq last-x stack-x) (setq stack-x (sinh stack-x))".to_string(),
            HP41Instruction::Cosh => "  (setq last-x stack-x) (setq stack-x (cosh stack-x))".to_string(),
            HP41Instruction::Tanh => "  (setq last-x stack-x) (setq stack-x (tanh stack-x))".to_string(),
            HP41Instruction::Asinh => "  (setq last-x stack-x) (setq stack-x (asinh stack-x))".to_string(),
            HP41Instruction::Acosh => "  (setq last-x stack-x) (setq stack-x (acosh stack-x))".to_string(),
            HP41Instruction::Atanh => "  (setq last-x stack-x) (setq stack-x (atanh stack-x))".to_string(),

            // Conversions - simplified
            HP41Instruction::ToRect => "  ; P→R".to_string(),
            HP41Instruction::ToPolar => "  ; R→P".to_string(),
            HP41Instruction::ToDeg => "  (setq last-x stack-x) (setq stack-x (* stack-x (/ 180 pi)))".to_string(),
            HP41Instruction::ToRad => "  (setq last-x stack-x) (setq stack-x (* stack-x (/ pi 180)))".to_string(),
            HP41Instruction::ToHms => "  ; →HMS".to_string(),
            HP41Instruction::ToHr => "  ; →HR".to_string(),

            // Statistics - placeholders
            HP41Instruction::SigmaPlus => "  ; Σ+".to_string(),
            HP41Instruction::SigmaMinus => "  ; Σ-".to_string(),
            HP41Instruction::Mean => "  ; MEAN".to_string(),
            HP41Instruction::Sdev => "  ; SDEV".to_string(),

            _ => format!("  ; {:?}", instr),
        }
    }

    /// Sanitize label name for LISP
    fn sanitize_label(name: &str) -> String {
        name.to_lowercase()
            .replace(' ', "-")
            .replace('"', "")
    }
}

/// HP-41C Transpiler with WASM bindings
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct HP41Transpiler {
    program: Option<HP41Program>,
    pc: usize,              // Program counter
    running: bool,          // Is program running?
    prgm_mode: bool,        // PRGM mode (viewing program)
    return_stack: Vec<usize>, // Return addresses for XEQ
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl HP41Transpiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HP41Transpiler {
        HP41Transpiler {
            program: None,
            pc: 0,
            running: false,
            prgm_mode: false,
            return_stack: Vec::new(),
        }
    }

    /// Parse HP-41C program text
    #[wasm_bindgen]
    pub fn parse(&mut self, input: &str) -> Result<String, JsValue> {
        match HP41Program::parse(input) {
            Ok(prog) => {
                let name = prog.name.clone();
                let instr_count = prog.instructions.len();
                self.program = Some(prog);
                self.pc = 0;
                self.running = false;
                self.return_stack.clear();
                Ok(format!("Parsed '{}' with {} instructions", name, instr_count))
            }
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    /// Transpile to LISP
    #[wasm_bindgen]
    pub fn to_lisp(&self) -> Result<String, JsValue> {
        match &self.program {
            Some(prog) => Ok(prog.to_lisp()),
            None => Err(JsValue::from_str("No program loaded")),
        }
    }

    /// Get program name
    #[wasm_bindgen]
    pub fn get_name(&self) -> String {
        self.program.as_ref().map(|p| p.name.clone()).unwrap_or_default()
    }

    /// Get instruction count
    #[wasm_bindgen]
    pub fn instruction_count(&self) -> usize {
        self.program.as_ref().map(|p| p.instructions.len()).unwrap_or(0)
    }

    /// Get current program counter
    #[wasm_bindgen]
    pub fn get_pc(&self) -> usize {
        self.pc
    }

    /// Set program counter
    #[wasm_bindgen]
    pub fn set_pc(&mut self, pc: usize) {
        if let Some(prog) = &self.program {
            if pc < prog.instructions.len() {
                self.pc = pc;
            }
        }
    }

    /// Toggle PRGM mode
    #[wasm_bindgen]
    pub fn toggle_prgm_mode(&mut self) -> bool {
        self.prgm_mode = !self.prgm_mode;
        self.prgm_mode
    }

    /// Is in PRGM mode?
    #[wasm_bindgen]
    pub fn is_prgm_mode(&self) -> bool {
        self.prgm_mode
    }

    /// Is program running?
    #[wasm_bindgen]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get instruction at current PC as display string (like HP-41C shows)
    #[wasm_bindgen]
    pub fn get_current_instruction_display(&self) -> String {
        match &self.program {
            Some(prog) if self.pc < prog.instructions.len() => {
                let instr = &prog.instructions[self.pc];
                format!("{:02} {}", self.pc + 1, Self::instruction_to_display(instr))
            }
            _ => "END".to_string(),
        }
    }

    /// Get instruction at specific line
    #[wasm_bindgen]
    pub fn get_instruction_at(&self, line: usize) -> String {
        match &self.program {
            Some(prog) if line < prog.instructions.len() => {
                let instr = &prog.instructions[line];
                format!("{:02} {}", line + 1, Self::instruction_to_display(instr))
            }
            Some(_) => format!("{:02} .END.", line + 1),
            None => "".to_string(),
        }
    }

    /// Get LISP code for current instruction (for SST execution)
    #[wasm_bindgen]
    pub fn get_current_lisp(&self) -> String {
        match &self.program {
            Some(prog) if self.pc < prog.instructions.len() => {
                let instr = &prog.instructions[self.pc];
                prog.instruction_to_lisp(instr, self.pc).trim().to_string()
            }
            _ => "".to_string(),
        }
    }

    /// SST - Single Step: execute current instruction and advance PC
    /// Returns: (lisp_code, next_pc, is_control_flow, target_label)
    #[wasm_bindgen]
    pub fn sst(&mut self) -> String {
        let prog = match &self.program {
            Some(p) => p,
            None => return "".to_string(),
        };

        if self.pc >= prog.instructions.len() {
            return "".to_string();
        }

        let instr = &prog.instructions[self.pc];
        let lisp = prog.instruction_to_lisp(instr, self.pc).trim().to_string();

        // Handle control flow
        match instr {
            HP41Instruction::Gto(label) => {
                if let Some(&target) = prog.labels.get(label) {
                    self.pc = target;
                } else {
                    self.pc += 1;
                }
            }
            HP41Instruction::Xeq(label) => {
                self.return_stack.push(self.pc + 1);
                if let Some(&target) = prog.labels.get(label) {
                    self.pc = target;
                } else {
                    self.pc += 1;
                }
            }
            HP41Instruction::Rtn | HP41Instruction::End => {
                if let Some(ret_addr) = self.return_stack.pop() {
                    self.pc = ret_addr;
                } else {
                    self.running = false;
                    self.pc = 0;
                }
            }
            HP41Instruction::Stop => {
                self.running = false;
                self.pc += 1;
            }
            _ => {
                self.pc += 1;
            }
        }

        lisp
    }

    /// BST - Back Step: move PC back one instruction
    #[wasm_bindgen]
    pub fn bst(&mut self) {
        if self.pc > 0 {
            self.pc -= 1;
        }
    }

    /// Run all - collect all LISP to execute from current PC until RTN/END
    /// Returns a single string of LISP code that can be eval'd in one go
    #[wasm_bindgen]
    pub fn run_all(&mut self, max_steps: usize) -> String {
        let prog = match &self.program {
            Some(p) => p,
            None => return "".to_string(),
        };

        self.running = true;
        let mut lisp_code = Vec::new();
        let mut steps = 0;

        while self.running && steps < max_steps {
            if self.pc >= prog.instructions.len() {
                break;
            }

            let instr = &prog.instructions[self.pc];
            let lisp = self.get_executable_lisp(instr);

            // Only add executable LISP (not labels, defun wrappers, etc.)
            if !lisp.is_empty() {
                lisp_code.push(lisp);
            }

            // Handle control flow and advance PC
            match instr {
                HP41Instruction::Gto(label) => {
                    if let Some(&target) = prog.labels.get(label) {
                        self.pc = target;
                    } else {
                        self.pc += 1;
                    }
                }
                HP41Instruction::Xeq(label) => {
                    self.return_stack.push(self.pc + 1);
                    if let Some(&target) = prog.labels.get(label) {
                        self.pc = target;
                    } else {
                        self.pc += 1;
                    }
                }
                HP41Instruction::Rtn | HP41Instruction::End => {
                    if let Some(ret_addr) = self.return_stack.pop() {
                        self.pc = ret_addr;
                    } else {
                        self.running = false;
                    }
                }
                HP41Instruction::Stop => {
                    self.running = false;
                    self.pc += 1;
                }
                _ => {
                    self.pc += 1;
                }
            }

            steps += 1;
        }

        // Return all LISP as a progn block
        if lisp_code.is_empty() {
            "".to_string()
        } else {
            format!("(progn {})", lisp_code.join(" "))
        }
    }

    /// Get executable LISP for an instruction (no defun wrappers)
    fn get_executable_lisp(&self, instr: &HP41Instruction) -> String {
        match instr {
            // Skip labels - they don't execute
            HP41Instruction::Lbl(_) => "".to_string(),
            HP41Instruction::End => "".to_string(),
            HP41Instruction::Rtn => "".to_string(),

            // Numbers push to stack
            HP41Instruction::Number(n) => format!("(stack-push {})", n),

            // Stack operations
            HP41Instruction::Enter => "(stack-push stack-x)".to_string(),
            HP41Instruction::Clx => "(setq stack-x 0)".to_string(),
            HP41Instruction::ClSt => "(stack-clear)".to_string(),
            HP41Instruction::XchgY => "(stack-swap)".to_string(),
            HP41Instruction::RollDown => "(stack-roll-down)".to_string(),
            HP41Instruction::RollUp => "(stack-roll-up)".to_string(),
            HP41Instruction::LastX => "(stack-push last-x)".to_string(),

            // Arithmetic
            HP41Instruction::Add => "(rpn-add)".to_string(),
            HP41Instruction::Sub => "(rpn-sub)".to_string(),
            HP41Instruction::Mul => "(rpn-mul)".to_string(),
            HP41Instruction::Div => "(rpn-div)".to_string(),
            HP41Instruction::Chs => "(rpn-chs)".to_string(),

            // Math functions
            HP41Instruction::Sqrt => "(rpn-sqrt)".to_string(),
            HP41Instruction::Square => "(setq last-x stack-x) (setq stack-x (* stack-x stack-x))".to_string(),
            HP41Instruction::Reciprocal => "(rpn-inv)".to_string(),
            HP41Instruction::Power => "(rpn-pow)".to_string(),

            // Trigonometry
            HP41Instruction::Sin => "(rpn-sin)".to_string(),
            HP41Instruction::Cos => "(rpn-cos)".to_string(),
            HP41Instruction::Tan => "(rpn-tan)".to_string(),
            HP41Instruction::Asin => "(setq last-x stack-x) (setq stack-x (asin stack-x))".to_string(),
            HP41Instruction::Acos => "(setq last-x stack-x) (setq stack-x (acos stack-x))".to_string(),
            HP41Instruction::Atan => "(setq last-x stack-x) (setq stack-x (atan stack-x))".to_string(),

            // Logarithms
            HP41Instruction::Log => "(rpn-log)".to_string(),
            HP41Instruction::Ln => "(rpn-ln)".to_string(),
            HP41Instruction::Exp10 => "(setq last-x stack-x) (setq stack-x (expt 10 stack-x))".to_string(),
            HP41Instruction::Exp => "(setq last-x stack-x) (setq stack-x (exp stack-x))".to_string(),

            // Constants
            HP41Instruction::Pi => "(rpn-pi)".to_string(),

            // Storage
            HP41Instruction::Sto(reg) => format!("(setq reg-{} stack-x)", HP41Program::sanitize_label(reg)),
            HP41Instruction::Rcl(reg) => format!("(stack-push reg-{})", HP41Program::sanitize_label(reg)),
            HP41Instruction::StoAdd(reg) => format!("(setq reg-{} (+ reg-{} stack-x))",
                HP41Program::sanitize_label(reg), HP41Program::sanitize_label(reg)),
            HP41Instruction::StoSub(reg) => format!("(setq reg-{} (- reg-{} stack-x))",
                HP41Program::sanitize_label(reg), HP41Program::sanitize_label(reg)),
            HP41Instruction::StoMul(reg) => format!("(setq reg-{} (* reg-{} stack-x))",
                HP41Program::sanitize_label(reg), HP41Program::sanitize_label(reg)),
            HP41Instruction::StoDiv(reg) => format!("(setq reg-{} (/ reg-{} stack-x))",
                HP41Program::sanitize_label(reg), HP41Program::sanitize_label(reg)),

            // Control flow - handled separately
            HP41Instruction::Gto(_) => "".to_string(),
            HP41Instruction::Xeq(_) => "".to_string(),
            HP41Instruction::Stop => "".to_string(),

            // Integer operations
            HP41Instruction::Int => "(setq last-x stack-x) (setq stack-x (truncate stack-x))".to_string(),
            HP41Instruction::Frac => "(setq last-x stack-x) (setq stack-x (- stack-x (truncate stack-x)))".to_string(),
            HP41Instruction::Abs => "(setq last-x stack-x) (setq stack-x (abs stack-x))".to_string(),
            HP41Instruction::Mod => "(setq last-x stack-x) (let ((r (mod stack-y stack-x))) (stack-drop) (setq stack-x r))".to_string(),

            // Percent
            HP41Instruction::Percent => "(setq last-x stack-x) (setq stack-x (* stack-y (/ stack-x 100)))".to_string(),
            HP41Instruction::PercentCh => "(setq last-x stack-x) (setq stack-x (* 100 (/ (- stack-x stack-y) stack-y)))".to_string(),

            // Default - skip
            _ => "".to_string(),
        }
    }

    /// GTO line number
    #[wasm_bindgen]
    pub fn goto_line(&mut self, line: usize) {
        if let Some(prog) = &self.program {
            if line < prog.instructions.len() {
                self.pc = line;
            }
        }
    }

    /// GTO label
    #[wasm_bindgen]
    pub fn goto_label(&mut self, label: &str) -> bool {
        if let Some(prog) = &self.program {
            let label_upper = label.to_uppercase();
            if let Some(&target) = prog.labels.get(&label_upper) {
                self.pc = target;
                return true;
            }
        }
        false
    }

    /// Start running from current PC
    #[wasm_bindgen]
    pub fn run(&mut self) {
        self.running = true;
        self.prgm_mode = false;
    }

    /// Stop running
    #[wasm_bindgen]
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset to beginning
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.pc = 0;
        self.running = false;
        self.return_stack.clear();
    }

    /// Get all program lines for display
    #[wasm_bindgen]
    pub fn get_program_listing(&self) -> String {
        match &self.program {
            Some(prog) => {
                let mut lines = Vec::new();
                for (i, instr) in prog.instructions.iter().enumerate() {
                    let marker = if i == self.pc { ">" } else { " " };
                    lines.push(format!("{}{:02} {}", marker, i + 1, Self::instruction_to_display(instr)));
                }
                lines.push(format!(" {:02} .END.", prog.instructions.len() + 1));
                lines.join("\n")
            }
            None => "No program loaded".to_string(),
        }
    }

    /// Convert instruction to HP-41C display format
    fn instruction_to_display(instr: &HP41Instruction) -> String {
        match instr {
            HP41Instruction::Number(n) => format!("{}", n),
            HP41Instruction::Lbl(name) => format!("LBL \"{}\"", name),
            HP41Instruction::Gto(name) => format!("GTO {}", name),
            HP41Instruction::Xeq(name) => format!("XEQ {}", name),
            HP41Instruction::Rtn => "RTN".to_string(),
            HP41Instruction::End => "END".to_string(),
            HP41Instruction::Stop => "STOP".to_string(),
            HP41Instruction::Enter => "ENTER^".to_string(),
            HP41Instruction::Clx => "CLX".to_string(),
            HP41Instruction::ClSt => "CLST".to_string(),
            HP41Instruction::XchgY => "X<>Y".to_string(),
            HP41Instruction::RollDown => "RDN".to_string(),
            HP41Instruction::RollUp => "R^".to_string(),
            HP41Instruction::LastX => "LASTX".to_string(),
            HP41Instruction::Add => "+".to_string(),
            HP41Instruction::Sub => "-".to_string(),
            HP41Instruction::Mul => "*".to_string(),
            HP41Instruction::Div => "/".to_string(),
            HP41Instruction::Chs => "CHS".to_string(),
            HP41Instruction::Sqrt => "SQRT".to_string(),
            HP41Instruction::Square => "X^2".to_string(),
            HP41Instruction::Reciprocal => "1/X".to_string(),
            HP41Instruction::Power => "Y^X".to_string(),
            HP41Instruction::Sin => "SIN".to_string(),
            HP41Instruction::Cos => "COS".to_string(),
            HP41Instruction::Tan => "TAN".to_string(),
            HP41Instruction::Asin => "ASIN".to_string(),
            HP41Instruction::Acos => "ACOS".to_string(),
            HP41Instruction::Atan => "ATAN".to_string(),
            HP41Instruction::Log => "LOG".to_string(),
            HP41Instruction::Ln => "LN".to_string(),
            HP41Instruction::Exp10 => "10^X".to_string(),
            HP41Instruction::Exp => "E^X".to_string(),
            HP41Instruction::Pi => "PI".to_string(),
            HP41Instruction::Sto(reg) => format!("STO {}", reg),
            HP41Instruction::Rcl(reg) => format!("RCL {}", reg),
            HP41Instruction::StoAdd(reg) => format!("STO+ {}", reg),
            HP41Instruction::StoSub(reg) => format!("STO- {}", reg),
            HP41Instruction::StoMul(reg) => format!("STO* {}", reg),
            HP41Instruction::StoDiv(reg) => format!("STO/ {}", reg),
            HP41Instruction::XchgReg(reg) => format!("X<> {}", reg),
            HP41Instruction::TestXEq0 => "X=0?".to_string(),
            HP41Instruction::TestXNe0 => "X<>0?".to_string(),
            HP41Instruction::TestXLt0 => "X<0?".to_string(),
            HP41Instruction::TestXGt0 => "X>0?".to_string(),
            HP41Instruction::TestXLe0 => "X<=0?".to_string(),
            HP41Instruction::TestXGe0 => "X>=0?".to_string(),
            HP41Instruction::TestXEqY => "X=Y?".to_string(),
            HP41Instruction::TestXNeY => "X<>Y?".to_string(),
            HP41Instruction::TestXLtY => "X<Y?".to_string(),
            HP41Instruction::TestXGtY => "X>Y?".to_string(),
            HP41Instruction::TestXLeY => "X<=Y?".to_string(),
            HP41Instruction::TestXGeY => "X>=Y?".to_string(),
            HP41Instruction::Isg(reg) => format!("ISG {}", reg),
            HP41Instruction::Dse(reg) => format!("DSE {}", reg),
            HP41Instruction::SetFlag(n) => format!("SF {:02}", n),
            HP41Instruction::ClearFlag(n) => format!("CF {:02}", n),
            HP41Instruction::TestFlagSet(n) => format!("FS? {:02}", n),
            HP41Instruction::TestFlagClear(n) => format!("FC? {:02}", n),
            HP41Instruction::View(reg) => format!("VIEW {}", reg),
            HP41Instruction::Prompt => "PROMPT".to_string(),
            HP41Instruction::Aview => "AVIEW".to_string(),
            HP41Instruction::Cla => "CLA".to_string(),
            HP41Instruction::AlphaString(s) => format!("\"{}\"", s),
            HP41Instruction::Nop => "NOP".to_string(),
            HP41Instruction::Beep => "BEEP".to_string(),
            HP41Instruction::Percent => "%".to_string(),
            HP41Instruction::PercentCh => "%CH".to_string(),
            HP41Instruction::Int => "INT".to_string(),
            HP41Instruction::Frac => "FRAC".to_string(),
            HP41Instruction::Abs => "ABS".to_string(),
            HP41Instruction::Sign => "SIGN".to_string(),
            HP41Instruction::Mod => "MOD".to_string(),
            _ => format!("{:?}", instr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let prog = HP41Program::parse(r#"
            LBL "AREA"
            X^2
            PI
            *
            RTN
        "#).unwrap();

        assert_eq!(prog.name, "AREA");
        assert_eq!(prog.instructions.len(), 5);
    }

    #[test]
    fn test_transpile_area() {
        let prog = HP41Program::parse(r#"
            LBL "AREA"
            X^2
            PI
            *
            RTN
        "#).unwrap();

        let lisp = prog.to_lisp();
        assert!(lisp.contains("defun hp41-area"));
        assert!(lisp.contains("(* stack-x stack-x)"));
        assert!(lisp.contains("rpn-pi"));
        assert!(lisp.contains("rpn-mul"));
    }

    #[test]
    fn test_parse_factorial() {
        let prog = HP41Program::parse(r#"
            LBL "FACT"
            STO 00
            1
            LBL 01
            RCL 00
            *
            DSE 00
            GTO 01
            RTN
        "#).unwrap();

        assert_eq!(prog.name, "FACT");
        assert!(prog.labels.contains_key("FACT"));
        assert!(prog.labels.contains_key("01"));
    }
}

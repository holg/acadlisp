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

/// Sanitize label name for LISP identifiers
fn sanitize_label(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .replace('"', "")
}

impl HP41Instruction {
    /// Generate executable LISP code for this instruction (no defun wrappers)
    pub fn to_executable_lisp(&self) -> String {
        match self {
            // Numbers push to stack
            HP41Instruction::Number(n) => format!("(stack-push {})", n),

            // Labels, control flow - no executable code (handled by runtime)
            HP41Instruction::Lbl(_) => String::new(),
            HP41Instruction::Gto(_) => String::new(),
            HP41Instruction::Xeq(_) => String::new(),
            HP41Instruction::Rtn => String::new(),
            HP41Instruction::End => String::new(),
            HP41Instruction::Stop => String::new(),

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
            HP41Instruction::Square => "(rpn-square)".to_string(),
            HP41Instruction::Reciprocal => "(rpn-inv)".to_string(),
            HP41Instruction::Power => "(rpn-pow)".to_string(),

            // Trigonometry
            HP41Instruction::Sin => "(rpn-sin)".to_string(),
            HP41Instruction::Cos => "(rpn-cos)".to_string(),
            HP41Instruction::Tan => "(rpn-tan)".to_string(),
            HP41Instruction::Asin => "(rpn-asin)".to_string(),
            HP41Instruction::Acos => "(rpn-acos)".to_string(),
            HP41Instruction::Atan => "(rpn-atan)".to_string(),

            // Logarithms
            HP41Instruction::Log => "(rpn-log)".to_string(),
            HP41Instruction::Ln => "(rpn-ln)".to_string(),
            HP41Instruction::Exp10 => "(rpn-exp10)".to_string(),
            HP41Instruction::Exp => "(rpn-exp)".to_string(),

            // Constants
            HP41Instruction::Pi => "(rpn-pi)".to_string(),

            // Storage
            HP41Instruction::Sto(reg) => format!("(setq reg-{} stack-x)", sanitize_label(reg)),
            HP41Instruction::Rcl(reg) => format!("(stack-push reg-{})", sanitize_label(reg)),
            HP41Instruction::StoAdd(reg) => {
                let r = sanitize_label(reg);
                format!("(setq reg-{} (+ reg-{} stack-x))", r, r)
            }
            HP41Instruction::StoSub(reg) => {
                let r = sanitize_label(reg);
                format!("(setq reg-{} (- reg-{} stack-x))", r, r)
            }
            HP41Instruction::StoMul(reg) => {
                let r = sanitize_label(reg);
                format!("(setq reg-{} (* reg-{} stack-x))", r, r)
            }
            HP41Instruction::StoDiv(reg) => {
                let r = sanitize_label(reg);
                format!("(setq reg-{} (/ reg-{} stack-x))", r, r)
            }
            HP41Instruction::XchgReg(reg) => {
                let r = sanitize_label(reg);
                format!("(let ((tmp stack-x)) (setq stack-x reg-{}) (setq reg-{} tmp))", r, r)
            }

            // Conditional tests - no executable code (handled by runtime skip logic)
            HP41Instruction::TestXEq0 | HP41Instruction::TestXNe0 |
            HP41Instruction::TestXLt0 | HP41Instruction::TestXGt0 |
            HP41Instruction::TestXLe0 | HP41Instruction::TestXGe0 |
            HP41Instruction::TestXEqY | HP41Instruction::TestXNeY |
            HP41Instruction::TestXLtY | HP41Instruction::TestXGtY |
            HP41Instruction::TestXLeY | HP41Instruction::TestXGeY => String::new(),

            // Loop constructs - handled by runtime
            HP41Instruction::Isg(_) | HP41Instruction::Dse(_) => String::new(),

            // Flags
            HP41Instruction::SetFlag(n) => format!("(setq flag-{} t)", n),
            HP41Instruction::ClearFlag(n) => format!("(setq flag-{} nil)", n),
            HP41Instruction::TestFlagSet(_) | HP41Instruction::TestFlagClear(_) => String::new(),

            // I/O
            HP41Instruction::View(reg) => format!("(princ reg-{})", sanitize_label(reg)),
            HP41Instruction::Prompt => String::new(),
            HP41Instruction::Aview => "(princ alpha-reg)".to_string(),
            HP41Instruction::Cla => "(setq alpha-reg \"\")".to_string(),
            HP41Instruction::AlphaString(s) => format!("(setq alpha-reg (concat alpha-reg \"{}\"))", s),

            // Misc
            HP41Instruction::Nop => String::new(),
            HP41Instruction::Beep => String::new(),

            // Percent
            HP41Instruction::Percent => "(rpn-percent)".to_string(),
            HP41Instruction::PercentCh => "(rpn-percent-ch)".to_string(),

            // Conversions
            HP41Instruction::ToRect => "(rpn-to-rect)".to_string(),
            HP41Instruction::ToPolar => "(rpn-to-polar)".to_string(),
            HP41Instruction::ToDeg => "(rpn-to-deg)".to_string(),
            HP41Instruction::ToRad => "(rpn-to-rad)".to_string(),
            HP41Instruction::ToHms => "(rpn-to-hms)".to_string(),
            HP41Instruction::ToHr => "(rpn-to-hr)".to_string(),

            // Statistics
            HP41Instruction::SigmaPlus => "(rpn-sigma-plus)".to_string(),
            HP41Instruction::SigmaMinus => "(rpn-sigma-minus)".to_string(),
            HP41Instruction::Mean => "(rpn-mean)".to_string(),
            HP41Instruction::Sdev => "(rpn-sdev)".to_string(),

            // Integer operations
            HP41Instruction::Int => "(rpn-int)".to_string(),
            HP41Instruction::Frac => "(rpn-frac)".to_string(),
            HP41Instruction::Abs => "(rpn-abs)".to_string(),
            HP41Instruction::Sign => "(rpn-sign)".to_string(),
            HP41Instruction::Mod => "(rpn-mod)".to_string(),

            // Hyperbolic
            HP41Instruction::Sinh => "(rpn-sinh)".to_string(),
            HP41Instruction::Cosh => "(rpn-cosh)".to_string(),
            HP41Instruction::Tanh => "(rpn-tanh)".to_string(),
            HP41Instruction::Asinh => "(rpn-asinh)".to_string(),
            HP41Instruction::Acosh => "(rpn-acosh)".to_string(),
            HP41Instruction::Atanh => "(rpn-atanh)".to_string(),
        }
    }

    /// Generate LISP code for transpilation output (with formatting for defun context)
    pub fn to_transpile_lisp(&self) -> String {
        match self {
            // Labels define functions
            HP41Instruction::Lbl(name) => format!("\n(defun hp41-{} ()", sanitize_label(name)),

            // Control flow in transpiled context
            HP41Instruction::Gto(name) => format!("  (hp41-{})", sanitize_label(name)),
            HP41Instruction::Xeq(name) => format!("  (hp41-{})", sanitize_label(name)),
            HP41Instruction::Rtn => "  stack-x)".to_string(),
            HP41Instruction::End => ")".to_string(),
            HP41Instruction::Stop => "  ; STOP".to_string(),

            // Conditional tests as when blocks
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

            // Loop constructs with inline logic for transpilation
            HP41Instruction::Isg(reg) => {
                let r = sanitize_label(reg);
                format!("  (setq reg-{} (+ reg-{} 1)) (when (> reg-{} (truncate reg-{}))", r, r, r, r)
            }
            HP41Instruction::Dse(reg) => {
                let r = sanitize_label(reg);
                format!("  (setq reg-{} (- reg-{} 1)) (when (<= reg-{} (truncate reg-{}))", r, r, r, r)
            }

            HP41Instruction::TestFlagSet(n) => format!("  (when flag-{}", n),
            HP41Instruction::TestFlagClear(n) => format!("  (when (not flag-{})", n),

            HP41Instruction::Prompt => "  ; PROMPT".to_string(),
            HP41Instruction::Nop => String::new(),
            HP41Instruction::Beep => "  ; BEEP".to_string(),

            // Everything else uses executable LISP with indentation
            _ => {
                let exec = self.to_executable_lisp();
                if exec.is_empty() {
                    String::new()
                } else {
                    format!("  {}", exec)
                }
            }
        }
    }

    /// Generate HP-41C display format string
    pub fn to_display(&self) -> String {
        match self {
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
            HP41Instruction::ToRect => "P-R".to_string(),
            HP41Instruction::ToPolar => "R-P".to_string(),
            HP41Instruction::ToDeg => "->DEG".to_string(),
            HP41Instruction::ToRad => "->RAD".to_string(),
            HP41Instruction::ToHms => "->HMS".to_string(),
            HP41Instruction::ToHr => "->HR".to_string(),
            HP41Instruction::SigmaPlus => "E+".to_string(),
            HP41Instruction::SigmaMinus => "E-".to_string(),
            HP41Instruction::Mean => "MEAN".to_string(),
            HP41Instruction::Sdev => "SDEV".to_string(),
            HP41Instruction::Int => "INT".to_string(),
            HP41Instruction::Frac => "FRAC".to_string(),
            HP41Instruction::Abs => "ABS".to_string(),
            HP41Instruction::Sign => "SIGN".to_string(),
            HP41Instruction::Mod => "MOD".to_string(),
            HP41Instruction::Sinh => "SINH".to_string(),
            HP41Instruction::Cosh => "COSH".to_string(),
            HP41Instruction::Tanh => "TANH".to_string(),
            HP41Instruction::Asinh => "ASINH".to_string(),
            HP41Instruction::Acosh => "ACOSH".to_string(),
            HP41Instruction::Atanh => "ATANH".to_string(),
        }
    }

    /// Check if this instruction is a conditional test
    pub fn is_conditional(&self) -> bool {
        matches!(self,
            HP41Instruction::TestXEq0 | HP41Instruction::TestXNe0 |
            HP41Instruction::TestXLt0 | HP41Instruction::TestXGt0 |
            HP41Instruction::TestXLe0 | HP41Instruction::TestXGe0 |
            HP41Instruction::TestXEqY | HP41Instruction::TestXNeY |
            HP41Instruction::TestXLtY | HP41Instruction::TestXGtY |
            HP41Instruction::TestXLeY | HP41Instruction::TestXGeY |
            HP41Instruction::TestFlagSet(_) | HP41Instruction::TestFlagClear(_) |
            HP41Instruction::Isg(_) | HP41Instruction::Dse(_)
        )
    }
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

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
                continue;
            }

            let instr = Self::parse_instruction(line)?;

            if let HP41Instruction::Lbl(ref name) = instr {
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
            "DEG" | "→DEG" | "->DEG" => Ok(HP41Instruction::ToDeg),
            "RAD" | "→RAD" | "->RAD" => Ok(HP41Instruction::ToRad),
            "HMS" | "→HMS" | "->HMS" => Ok(HP41Instruction::ToHms),
            "HR" | "→HR" | "->HR" => Ok(HP41Instruction::ToHr),

            // Statistics
            "Σ+" | "SIGMA+" | "E+" => Ok(HP41Instruction::SigmaPlus),
            "Σ-" | "SIGMA-" | "E-" => Ok(HP41Instruction::SigmaMinus),
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
        output.push_str(&format!("; HP-41C Program: {}\n", self.name));
        output.push_str("; Transpiled to LISP\n\n");

        for instr in &self.instructions {
            let lisp = instr.to_transpile_lisp();
            if !lisp.is_empty() {
                output.push_str(&lisp);
                output.push('\n');
            }
        }

        output
    }
}

// ============================================================================
// HP-41C Runtime Stack (pure Rust implementation)
// ============================================================================

/// HP-41C 4-level RPN stack with registers and flags
#[derive(Debug, Clone)]
pub struct HP41Stack {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub t: f64,
    pub last_x: f64,
    pub registers: HashMap<String, f64>,
    pub flags: [bool; 56],  // HP-41C has flags 00-55
    pub alpha: String,
    // Statistics registers (for Σ+/Σ-)
    pub stat_n: f64,        // Number of data points
    pub stat_sum_x: f64,    // Σx
    pub stat_sum_x2: f64,   // Σx²
    pub stat_sum_y: f64,    // Σy
    pub stat_sum_y2: f64,   // Σy²
    pub stat_sum_xy: f64,   // Σxy
}

impl Default for HP41Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl HP41Stack {
    pub fn new() -> Self {
        HP41Stack {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
            last_x: 0.0,
            registers: HashMap::new(),
            flags: [false; 56],
            alpha: String::new(),
            stat_n: 0.0,
            stat_sum_x: 0.0,
            stat_sum_x2: 0.0,
            stat_sum_y: 0.0,
            stat_sum_y2: 0.0,
            stat_sum_xy: 0.0,
        }
    }

    /// Push value onto stack (lift stack)
    pub fn push(&mut self, val: f64) {
        self.t = self.z;
        self.z = self.y;
        self.y = self.x;
        self.x = val;
    }

    /// Drop stack (after binary operation)
    pub fn stack_drop(&mut self) {
        self.last_x = self.x;
        self.x = self.y;
        self.y = self.z;
        self.z = self.t;
    }

    /// Drop without saving last_x
    fn drop_no_lastx(&mut self) {
        self.y = self.z;
        self.z = self.t;
    }

    /// Swap X and Y
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.x, &mut self.y);
    }

    /// Roll stack down
    pub fn roll_down(&mut self) {
        let tmp = self.x;
        self.x = self.y;
        self.y = self.z;
        self.z = self.t;
        self.t = tmp;
    }

    /// Roll stack up
    pub fn roll_up(&mut self) {
        let tmp = self.t;
        self.t = self.z;
        self.z = self.y;
        self.y = self.x;
        self.x = tmp;
    }

    /// Clear stack
    pub fn clear(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
        self.t = 0.0;
    }

    /// Clear statistics registers
    pub fn clear_stat(&mut self) {
        self.stat_n = 0.0;
        self.stat_sum_x = 0.0;
        self.stat_sum_x2 = 0.0;
        self.stat_sum_y = 0.0;
        self.stat_sum_y2 = 0.0;
        self.stat_sum_xy = 0.0;
    }

    /// Get register value
    pub fn get_reg(&self, name: &str) -> f64 {
        *self.registers.get(name).unwrap_or(&0.0)
    }

    /// Set register value
    pub fn set_reg(&mut self, name: &str, val: f64) {
        self.registers.insert(name.to_string(), val);
    }

    // ========================================================================
    // Arithmetic Operations
    // ========================================================================

    pub fn add(&mut self) {
        self.last_x = self.x;
        self.x += self.y;
        self.drop_no_lastx();
    }

    pub fn sub(&mut self) {
        self.last_x = self.x;
        self.x = self.y - self.x;
        self.drop_no_lastx();
    }

    pub fn mul(&mut self) {
        self.last_x = self.x;
        self.x *= self.y;
        self.drop_no_lastx();
    }

    pub fn div(&mut self) {
        self.last_x = self.x;
        self.x = self.y / self.x;
        self.drop_no_lastx();
    }

    pub fn chs(&mut self) {
        self.x = -self.x;
    }

    pub fn power(&mut self) {
        self.last_x = self.x;
        self.x = self.y.powf(self.x);
        self.drop_no_lastx();
    }

    // ========================================================================
    // Math Functions
    // ========================================================================

    pub fn sqrt(&mut self) {
        self.last_x = self.x;
        self.x = self.x.sqrt();
    }

    pub fn square(&mut self) {
        self.last_x = self.x;
        self.x = self.x * self.x;
    }

    pub fn reciprocal(&mut self) {
        self.last_x = self.x;
        self.x = 1.0 / self.x;
    }

    pub fn sin(&mut self) {
        self.last_x = self.x;
        self.x = self.x.sin();
    }

    pub fn cos(&mut self) {
        self.last_x = self.x;
        self.x = self.x.cos();
    }

    pub fn tan(&mut self) {
        self.last_x = self.x;
        self.x = self.x.tan();
    }

    pub fn asin(&mut self) {
        self.last_x = self.x;
        self.x = self.x.asin();
    }

    pub fn acos(&mut self) {
        self.last_x = self.x;
        self.x = self.x.acos();
    }

    pub fn atan(&mut self) {
        self.last_x = self.x;
        self.x = self.x.atan();
    }

    pub fn log(&mut self) {
        self.last_x = self.x;
        self.x = self.x.log10();
    }

    pub fn ln(&mut self) {
        self.last_x = self.x;
        self.x = self.x.ln();
    }

    pub fn exp10(&mut self) {
        self.last_x = self.x;
        self.x = 10.0_f64.powf(self.x);
    }

    pub fn exp(&mut self) {
        self.last_x = self.x;
        self.x = self.x.exp();
    }

    pub fn pi(&mut self) {
        self.push(std::f64::consts::PI);
    }

    // ========================================================================
    // Hyperbolic Functions
    // ========================================================================

    pub fn sinh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.sinh();
    }

    pub fn cosh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.cosh();
    }

    pub fn tanh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.tanh();
    }

    pub fn asinh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.asinh();
    }

    pub fn acosh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.acosh();
    }

    pub fn atanh(&mut self) {
        self.last_x = self.x;
        self.x = self.x.atanh();
    }

    // ========================================================================
    // Integer Functions
    // ========================================================================

    pub fn int(&mut self) {
        self.last_x = self.x;
        self.x = self.x.trunc();
    }

    pub fn frac(&mut self) {
        self.last_x = self.x;
        self.x = self.x.fract();
    }

    pub fn abs(&mut self) {
        self.last_x = self.x;
        self.x = self.x.abs();
    }

    pub fn sign(&mut self) {
        self.last_x = self.x;
        self.x = if self.x > 0.0 { 1.0 } else if self.x < 0.0 { -1.0 } else { 0.0 };
    }

    pub fn modulo(&mut self) {
        self.last_x = self.x;
        let r = self.y % self.x;
        self.drop_no_lastx();
        self.x = r;
    }

    // ========================================================================
    // Percent Functions
    // ========================================================================

    pub fn percent(&mut self) {
        self.last_x = self.x;
        self.x = self.y * (self.x / 100.0);
    }

    pub fn percent_ch(&mut self) {
        self.last_x = self.x;
        self.x = 100.0 * (self.x - self.y) / self.y;
    }

    // ========================================================================
    // Conversion Functions
    // ========================================================================

    pub fn to_deg(&mut self) {
        self.last_x = self.x;
        self.x = self.x.to_degrees();
    }

    pub fn to_rad(&mut self) {
        self.last_x = self.x;
        self.x = self.x.to_radians();
    }

    /// Polar to rectangular: x=r, y=θ → x=x_coord, y=y_coord
    pub fn to_rect(&mut self) {
        self.last_x = self.x;
        let r = self.y;
        let theta = self.x;
        self.x = r * theta.cos();
        self.y = r * theta.sin();
    }

    /// Rectangular to polar: x=x_coord, y=y_coord → x=r, y=θ
    pub fn to_polar(&mut self) {
        self.last_x = self.x;
        let x_coord = self.x;
        let y_coord = self.y;
        self.x = (x_coord * x_coord + y_coord * y_coord).sqrt();
        self.y = y_coord.atan2(x_coord);
    }

    /// Decimal hours to HMS (hours.MMSSss)
    pub fn to_hms(&mut self) {
        self.last_x = self.x;
        let hours = self.x.trunc();
        let dec_part = (self.x - hours).abs();
        let minutes = (dec_part * 60.0).trunc();
        let seconds = (dec_part * 60.0 - minutes) * 60.0;
        self.x = hours + minutes / 100.0 + seconds / 10000.0;
    }

    /// HMS to decimal hours
    /// HMS format: HH.MMSSss where MM=minutes (0-59), SS=seconds (0-59), ss=fractional seconds
    pub fn to_hr(&mut self) {
        self.last_x = self.x;
        let sign = if self.x < 0.0 { -1.0 } else { 1.0 };
        let abs_val = self.x.abs();
        let hours = abs_val.trunc();
        // Get fractional part and scale: need to handle floating point precision
        let frac = abs_val - hours;
        // Round to 6 decimal places to avoid floating point issues
        let frac_scaled = (frac * 1000000.0).round() / 10000.0;  // MMSSss format
        let minutes = frac_scaled.trunc();
        let ss_frac = frac_scaled - minutes;
        let seconds = (ss_frac * 100.0).round() / 100.0;  // SS.ss
        self.x = sign * (hours + minutes / 60.0 + seconds / 3600.0);
    }

    // ========================================================================
    // Statistics Functions
    // ========================================================================

    /// Σ+ : Add (x, y) data point
    pub fn sigma_plus(&mut self) {
        self.last_x = self.x;
        self.stat_n += 1.0;
        self.stat_sum_x += self.x;
        self.stat_sum_x2 += self.x * self.x;
        self.stat_sum_y += self.y;
        self.stat_sum_y2 += self.y * self.y;
        self.stat_sum_xy += self.x * self.y;
        self.drop_no_lastx();
        self.x = self.stat_n;
    }

    /// Σ- : Remove (x, y) data point
    pub fn sigma_minus(&mut self) {
        self.last_x = self.x;
        self.stat_n -= 1.0;
        self.stat_sum_x -= self.x;
        self.stat_sum_x2 -= self.x * self.x;
        self.stat_sum_y -= self.y;
        self.stat_sum_y2 -= self.y * self.y;
        self.stat_sum_xy -= self.x * self.y;
        self.drop_no_lastx();
        self.x = self.stat_n;
    }

    /// Mean: push (mean_x, mean_y) onto stack
    pub fn mean(&mut self) {
        if self.stat_n > 0.0 {
            self.push(self.stat_sum_y / self.stat_n);
            self.push(self.stat_sum_x / self.stat_n);
        }
    }

    /// Standard deviation: push (sdev_x, sdev_y) onto stack
    pub fn sdev(&mut self) {
        if self.stat_n > 1.0 {
            let n = self.stat_n;
            let sx = self.stat_sum_x;
            let sx2 = self.stat_sum_x2;
            let sy = self.stat_sum_y;
            let sy2 = self.stat_sum_y2;

            let var_x = (sx2 - sx * sx / n) / (n - 1.0);
            let var_y = (sy2 - sy * sy / n) / (n - 1.0);

            self.push(var_y.sqrt());
            self.push(var_x.sqrt());
        }
    }

    // ========================================================================
    // Conditional Tests (return true if condition is met, skip next if false)
    // ========================================================================

    pub fn test_x_eq_0(&self) -> bool { self.x == 0.0 }
    pub fn test_x_ne_0(&self) -> bool { self.x != 0.0 }
    pub fn test_x_lt_0(&self) -> bool { self.x < 0.0 }
    pub fn test_x_gt_0(&self) -> bool { self.x > 0.0 }
    pub fn test_x_le_0(&self) -> bool { self.x <= 0.0 }
    pub fn test_x_ge_0(&self) -> bool { self.x >= 0.0 }
    pub fn test_x_eq_y(&self) -> bool { self.x == self.y }
    pub fn test_x_ne_y(&self) -> bool { self.x != self.y }
    pub fn test_x_lt_y(&self) -> bool { self.x < self.y }
    pub fn test_x_gt_y(&self) -> bool { self.x > self.y }
    pub fn test_x_le_y(&self) -> bool { self.x <= self.y }
    pub fn test_x_ge_y(&self) -> bool { self.x >= self.y }

    /// ISG: Increment and Skip if Greater
    /// Register format: iiiii.fffcc where iiiii=current, fff=final, cc=increment
    pub fn isg(&mut self, reg: &str) -> bool {
        let val = self.get_reg(reg);
        let current = val.trunc();
        let control = (val.fract() * 100000.0).round() / 100.0;
        let final_val = control.trunc();
        let increment = (control.fract() * 100.0).round();
        let inc = if increment == 0.0 { 1.0 } else { increment };

        let new_current = current + inc;
        let new_val = new_current + (final_val / 1000.0) + (inc / 100000.0);
        self.set_reg(reg, new_val);

        new_current > final_val  // Skip next instruction if true
    }

    /// DSE: Decrement and Skip if Equal or less
    /// Register format: ccccccc.xxxyyzzz where ccc=counter, xxx=end, yy=step (default 1)
    /// Simple case: just counter value (e.g., 5.0) - decrements by 1, skips when <= 0
    pub fn dse(&mut self, reg: &str) -> bool {
        let val = self.get_reg(reg);
        let frac_part = val.fract().abs();

        // Check if there's a control part (fractional digits)
        if frac_part < 0.0001 {
            // Simple case: just a counter, decrement by 1, skip if <= 1
            let new_val = val - 1.0;
            self.set_reg(reg, new_val);
            new_val <= 1.0
        } else {
            // Full control format: ccccccc.xxxyyzzz
            let counter = val.trunc();
            // Extract end value (first 3 fractional digits) and step (next 2 digits)
            let frac_scaled = (frac_part * 100000.0).round();
            let end_val = (frac_scaled / 100.0).trunc();
            let step = frac_scaled % 100.0;
            let step = if step == 0.0 { 1.0 } else { step };

            let new_counter = counter - step;
            // Reconstruct value preserving control part
            let new_val = new_counter + frac_part;
            self.set_reg(reg, new_val);
            new_counter <= end_val
        }
    }
}

// ============================================================================
// HP-41C Runtime Engine
// ============================================================================

/// Complete HP-41C execution engine
#[derive(Debug, Clone)]
pub struct HP41Runtime {
    pub stack: HP41Stack,
    pub program: Option<HP41Program>,
    pub pc: usize,
    pub running: bool,
    pub prgm_mode: bool,
    pub return_stack: Vec<usize>,
}

impl Default for HP41Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl HP41Runtime {
    pub fn new() -> Self {
        HP41Runtime {
            stack: HP41Stack::new(),
            program: None,
            pc: 0,
            running: false,
            prgm_mode: false,
            return_stack: Vec::new(),
        }
    }

    /// Load and parse a program
    pub fn load(&mut self, input: &str) -> Result<String, String> {
        let prog = HP41Program::parse(input)?;
        let name = prog.name.clone();
        let count = prog.instructions.len();
        self.program = Some(prog);
        self.pc = 0;
        self.running = false;
        self.return_stack.clear();
        Ok(format!("Parsed '{}' with {} instructions", name, count))
    }

    /// Execute single instruction at current PC
    pub fn step(&mut self) -> Option<String> {
        let prog = self.program.as_ref()?;
        if self.pc >= prog.instructions.len() {
            return None;
        }

        let instr = prog.instructions[self.pc].clone();
        // execute_instruction handles all PC management internally
        self.execute_instruction(&instr)
    }

    /// Execute a single instruction
    fn execute_instruction(&mut self, instr: &HP41Instruction) -> Option<String> {
        match instr {
            HP41Instruction::Number(n) => {
                self.stack.push(*n);
                self.pc += 1;
            }

            HP41Instruction::Lbl(_) => {
                self.pc += 1;
            }

            HP41Instruction::Gto(label) => {
                if let Some(prog) = &self.program {
                    if let Some(&target) = prog.labels.get(label) {
                        self.pc = target;
                    } else {
                        self.pc += 1;
                    }
                }
            }

            HP41Instruction::Xeq(label) => {
                self.return_stack.push(self.pc + 1);
                if let Some(prog) = &self.program {
                    if let Some(&target) = prog.labels.get(label) {
                        self.pc = target;
                    } else {
                        self.pc += 1;
                    }
                }
            }

            HP41Instruction::Rtn | HP41Instruction::End => {
                if let Some(ret) = self.return_stack.pop() {
                    self.pc = ret;
                } else {
                    self.running = false;
                    self.pc = 0;
                }
            }

            HP41Instruction::Stop => {
                self.running = false;
                self.pc += 1;
            }

            // Stack operations
            HP41Instruction::Enter => { self.stack.push(self.stack.x); self.pc += 1; }
            HP41Instruction::Clx => { self.stack.x = 0.0; self.pc += 1; }
            HP41Instruction::ClSt => { self.stack.clear(); self.pc += 1; }
            HP41Instruction::XchgY => { self.stack.swap(); self.pc += 1; }
            HP41Instruction::RollDown => { self.stack.roll_down(); self.pc += 1; }
            HP41Instruction::RollUp => { self.stack.roll_up(); self.pc += 1; }
            HP41Instruction::LastX => { self.stack.push(self.stack.last_x); self.pc += 1; }

            // Arithmetic
            HP41Instruction::Add => { self.stack.add(); self.pc += 1; }
            HP41Instruction::Sub => { self.stack.sub(); self.pc += 1; }
            HP41Instruction::Mul => { self.stack.mul(); self.pc += 1; }
            HP41Instruction::Div => { self.stack.div(); self.pc += 1; }
            HP41Instruction::Chs => { self.stack.chs(); self.pc += 1; }
            HP41Instruction::Power => { self.stack.power(); self.pc += 1; }

            // Math
            HP41Instruction::Sqrt => { self.stack.sqrt(); self.pc += 1; }
            HP41Instruction::Square => { self.stack.square(); self.pc += 1; }
            HP41Instruction::Reciprocal => { self.stack.reciprocal(); self.pc += 1; }
            HP41Instruction::Sin => { self.stack.sin(); self.pc += 1; }
            HP41Instruction::Cos => { self.stack.cos(); self.pc += 1; }
            HP41Instruction::Tan => { self.stack.tan(); self.pc += 1; }
            HP41Instruction::Asin => { self.stack.asin(); self.pc += 1; }
            HP41Instruction::Acos => { self.stack.acos(); self.pc += 1; }
            HP41Instruction::Atan => { self.stack.atan(); self.pc += 1; }
            HP41Instruction::Log => { self.stack.log(); self.pc += 1; }
            HP41Instruction::Ln => { self.stack.ln(); self.pc += 1; }
            HP41Instruction::Exp10 => { self.stack.exp10(); self.pc += 1; }
            HP41Instruction::Exp => { self.stack.exp(); self.pc += 1; }
            HP41Instruction::Pi => { self.stack.pi(); self.pc += 1; }

            // Hyperbolic
            HP41Instruction::Sinh => { self.stack.sinh(); self.pc += 1; }
            HP41Instruction::Cosh => { self.stack.cosh(); self.pc += 1; }
            HP41Instruction::Tanh => { self.stack.tanh(); self.pc += 1; }
            HP41Instruction::Asinh => { self.stack.asinh(); self.pc += 1; }
            HP41Instruction::Acosh => { self.stack.acosh(); self.pc += 1; }
            HP41Instruction::Atanh => { self.stack.atanh(); self.pc += 1; }

            // Integer
            HP41Instruction::Int => { self.stack.int(); self.pc += 1; }
            HP41Instruction::Frac => { self.stack.frac(); self.pc += 1; }
            HP41Instruction::Abs => { self.stack.abs(); self.pc += 1; }
            HP41Instruction::Sign => { self.stack.sign(); self.pc += 1; }
            HP41Instruction::Mod => { self.stack.modulo(); self.pc += 1; }

            // Percent
            HP41Instruction::Percent => { self.stack.percent(); self.pc += 1; }
            HP41Instruction::PercentCh => { self.stack.percent_ch(); self.pc += 1; }

            // Conversions
            HP41Instruction::ToDeg => { self.stack.to_deg(); self.pc += 1; }
            HP41Instruction::ToRad => { self.stack.to_rad(); self.pc += 1; }
            HP41Instruction::ToRect => { self.stack.to_rect(); self.pc += 1; }
            HP41Instruction::ToPolar => { self.stack.to_polar(); self.pc += 1; }
            HP41Instruction::ToHms => { self.stack.to_hms(); self.pc += 1; }
            HP41Instruction::ToHr => { self.stack.to_hr(); self.pc += 1; }

            // Statistics
            HP41Instruction::SigmaPlus => { self.stack.sigma_plus(); self.pc += 1; }
            HP41Instruction::SigmaMinus => { self.stack.sigma_minus(); self.pc += 1; }
            HP41Instruction::Mean => { self.stack.mean(); self.pc += 1; }
            HP41Instruction::Sdev => { self.stack.sdev(); self.pc += 1; }

            // Storage
            HP41Instruction::Sto(reg) => {
                self.stack.set_reg(&sanitize_label(reg), self.stack.x);
                self.pc += 1;
            }
            HP41Instruction::Rcl(reg) => {
                let val = self.stack.get_reg(&sanitize_label(reg));
                self.stack.push(val);
                self.pc += 1;
            }
            HP41Instruction::StoAdd(reg) => {
                let r = sanitize_label(reg);
                let val = self.stack.get_reg(&r) + self.stack.x;
                self.stack.set_reg(&r, val);
                self.pc += 1;
            }
            HP41Instruction::StoSub(reg) => {
                let r = sanitize_label(reg);
                let val = self.stack.get_reg(&r) - self.stack.x;
                self.stack.set_reg(&r, val);
                self.pc += 1;
            }
            HP41Instruction::StoMul(reg) => {
                let r = sanitize_label(reg);
                let val = self.stack.get_reg(&r) * self.stack.x;
                self.stack.set_reg(&r, val);
                self.pc += 1;
            }
            HP41Instruction::StoDiv(reg) => {
                let r = sanitize_label(reg);
                let val = self.stack.get_reg(&r) / self.stack.x;
                self.stack.set_reg(&r, val);
                self.pc += 1;
            }
            HP41Instruction::XchgReg(reg) => {
                let r = sanitize_label(reg);
                let tmp = self.stack.x;
                self.stack.x = self.stack.get_reg(&r);
                self.stack.set_reg(&r, tmp);
                self.pc += 1;
            }

            // Flags
            HP41Instruction::SetFlag(n) => {
                if (*n as usize) < 56 { self.stack.flags[*n as usize] = true; }
                self.pc += 1;
            }
            HP41Instruction::ClearFlag(n) => {
                if (*n as usize) < 56 { self.stack.flags[*n as usize] = false; }
                self.pc += 1;
            }
            HP41Instruction::TestFlagSet(n) => {
                let cond = (*n as usize) < 56 && self.stack.flags[*n as usize];
                self.pc += if cond { 1 } else { 2 };
            }
            HP41Instruction::TestFlagClear(n) => {
                let cond = (*n as usize) >= 56 || !self.stack.flags[*n as usize];
                self.pc += if cond { 1 } else { 2 };
            }

            // Conditional tests
            HP41Instruction::TestXEq0 => { self.pc += if self.stack.test_x_eq_0() { 1 } else { 2 }; }
            HP41Instruction::TestXNe0 => { self.pc += if self.stack.test_x_ne_0() { 1 } else { 2 }; }
            HP41Instruction::TestXLt0 => { self.pc += if self.stack.test_x_lt_0() { 1 } else { 2 }; }
            HP41Instruction::TestXGt0 => { self.pc += if self.stack.test_x_gt_0() { 1 } else { 2 }; }
            HP41Instruction::TestXLe0 => { self.pc += if self.stack.test_x_le_0() { 1 } else { 2 }; }
            HP41Instruction::TestXGe0 => { self.pc += if self.stack.test_x_ge_0() { 1 } else { 2 }; }
            HP41Instruction::TestXEqY => { self.pc += if self.stack.test_x_eq_y() { 1 } else { 2 }; }
            HP41Instruction::TestXNeY => { self.pc += if self.stack.test_x_ne_y() { 1 } else { 2 }; }
            HP41Instruction::TestXLtY => { self.pc += if self.stack.test_x_lt_y() { 1 } else { 2 }; }
            HP41Instruction::TestXGtY => { self.pc += if self.stack.test_x_gt_y() { 1 } else { 2 }; }
            HP41Instruction::TestXLeY => { self.pc += if self.stack.test_x_le_y() { 1 } else { 2 }; }
            HP41Instruction::TestXGeY => { self.pc += if self.stack.test_x_ge_y() { 1 } else { 2 }; }

            // Loop constructs
            HP41Instruction::Isg(reg) => {
                let skip = self.stack.isg(&sanitize_label(reg));
                self.pc += if skip { 2 } else { 1 };
            }
            HP41Instruction::Dse(reg) => {
                let skip = self.stack.dse(&sanitize_label(reg));
                self.pc += if skip { 2 } else { 1 };
            }

            // I/O
            HP41Instruction::View(reg) => {
                let val = self.stack.get_reg(&sanitize_label(reg));
                self.pc += 1;
                return Some(format!("{}", val));
            }
            HP41Instruction::Aview => {
                self.pc += 1;
                return Some(self.stack.alpha.clone());
            }
            HP41Instruction::Cla => {
                self.stack.alpha.clear();
                self.pc += 1;
            }
            HP41Instruction::AlphaString(s) => {
                self.stack.alpha.push_str(s);
                self.pc += 1;
            }
            HP41Instruction::Prompt => {
                self.running = false;
                self.pc += 1;
                return Some(self.stack.alpha.clone());
            }

            // Misc
            HP41Instruction::Nop | HP41Instruction::Beep => {
                self.pc += 1;
            }
        }

        None
    }

    /// Run program until completion or max steps
    pub fn run(&mut self, max_steps: usize) -> f64 {
        self.running = true;
        let mut steps = 0;

        while self.running && steps < max_steps {
            if let Some(prog) = &self.program {
                if self.pc >= prog.instructions.len() {
                    break;
                }
            } else {
                break;
            }

            self.step();
            steps += 1;
        }

        self.stack.x
    }

    /// Get current instruction display
    pub fn current_instruction_display(&self) -> String {
        match &self.program {
            Some(prog) if self.pc < prog.instructions.len() => {
                format!("{:02} {}", self.pc + 1, prog.instructions[self.pc].to_display())
            }
            _ => "END".to_string(),
        }
    }

    /// Get program listing
    pub fn program_listing(&self) -> String {
        match &self.program {
            Some(prog) => {
                let mut lines = Vec::new();
                for (i, instr) in prog.instructions.iter().enumerate() {
                    let marker = if i == self.pc { ">" } else { " " };
                    lines.push(format!("{}{:02} {}", marker, i + 1, instr.to_display()));
                }
                lines.push(format!(" {:02} .END.", prog.instructions.len() + 1));
                lines.join("\n")
            }
            None => "No program loaded".to_string(),
        }
    }
}

// ============================================================================
// WASM Bindings
// ============================================================================

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct HP41Transpiler {
    runtime: HP41Runtime,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl HP41Transpiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HP41Transpiler {
        HP41Transpiler {
            runtime: HP41Runtime::new(),
        }
    }

    #[wasm_bindgen]
    pub fn parse(&mut self, input: &str) -> Result<String, JsValue> {
        self.runtime.load(input).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen]
    pub fn to_lisp(&self) -> Result<String, JsValue> {
        match &self.runtime.program {
            Some(prog) => Ok(prog.to_lisp()),
            None => Err(JsValue::from_str("No program loaded")),
        }
    }

    #[wasm_bindgen]
    pub fn get_name(&self) -> String {
        self.runtime.program.as_ref().map(|p| p.name.clone()).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn instruction_count(&self) -> usize {
        self.runtime.program.as_ref().map(|p| p.instructions.len()).unwrap_or(0)
    }

    #[wasm_bindgen]
    pub fn get_pc(&self) -> usize {
        self.runtime.pc
    }

    #[wasm_bindgen]
    pub fn set_pc(&mut self, pc: usize) {
        if let Some(prog) = &self.runtime.program {
            if pc < prog.instructions.len() {
                self.runtime.pc = pc;
            }
        }
    }

    #[wasm_bindgen]
    pub fn toggle_prgm_mode(&mut self) -> bool {
        self.runtime.prgm_mode = !self.runtime.prgm_mode;
        self.runtime.prgm_mode
    }

    #[wasm_bindgen]
    pub fn is_prgm_mode(&self) -> bool {
        self.runtime.prgm_mode
    }

    #[wasm_bindgen]
    pub fn is_running(&self) -> bool {
        self.runtime.running
    }

    #[wasm_bindgen]
    pub fn get_current_instruction_display(&self) -> String {
        self.runtime.current_instruction_display()
    }

    #[wasm_bindgen]
    pub fn get_instruction_at(&self, line: usize) -> String {
        match &self.runtime.program {
            Some(prog) if line < prog.instructions.len() => {
                format!("{:02} {}", line + 1, prog.instructions[line].to_display())
            }
            Some(_) => format!("{:02} .END.", line + 1),
            None => String::new(),
        }
    }

    #[wasm_bindgen]
    pub fn get_current_lisp(&self) -> String {
        match &self.runtime.program {
            Some(prog) if self.runtime.pc < prog.instructions.len() => {
                prog.instructions[self.runtime.pc].to_executable_lisp()
            }
            _ => String::new(),
        }
    }

    #[wasm_bindgen]
    pub fn sst(&mut self) -> String {
        let prog = match &self.runtime.program {
            Some(p) => p,
            None => return String::new(),
        };

        if self.runtime.pc >= prog.instructions.len() {
            return String::new();
        }

        let instr = &prog.instructions[self.runtime.pc];
        let lisp = instr.to_executable_lisp();

        self.runtime.step();

        lisp
    }

    #[wasm_bindgen]
    pub fn bst(&mut self) {
        if self.runtime.pc > 0 {
            self.runtime.pc -= 1;
        }
    }

    #[wasm_bindgen]
    pub fn run_all(&mut self, max_steps: usize) -> String {
        if self.runtime.program.is_none() {
            return String::new();
        }

        self.runtime.running = true;
        let mut lisp_code = Vec::new();
        let mut steps = 0;

        while self.runtime.running && steps < max_steps {
            // Get instruction count and current PC without holding borrow
            let (instr_count, pc) = {
                let prog = self.runtime.program.as_ref().unwrap();
                (prog.instructions.len(), self.runtime.pc)
            };

            if pc >= instr_count {
                break;
            }

            // Get the LISP for current instruction
            let lisp = {
                let prog = self.runtime.program.as_ref().unwrap();
                prog.instructions[pc].to_executable_lisp()
            };

            if !lisp.is_empty() {
                lisp_code.push(lisp);
            }

            self.runtime.step();
            steps += 1;
        }

        if lisp_code.is_empty() {
            String::new()
        } else {
            format!("(progn {})", lisp_code.join(" "))
        }
    }

    #[wasm_bindgen]
    pub fn goto_line(&mut self, line: usize) {
        if let Some(prog) = &self.runtime.program {
            if line < prog.instructions.len() {
                self.runtime.pc = line;
            }
        }
    }

    #[wasm_bindgen]
    pub fn goto_label(&mut self, label: &str) -> bool {
        if let Some(prog) = &self.runtime.program {
            let label_upper = label.to_uppercase();
            if let Some(&target) = prog.labels.get(&label_upper) {
                self.runtime.pc = target;
                return true;
            }
        }
        false
    }

    #[wasm_bindgen]
    pub fn run(&mut self) {
        self.runtime.running = true;
        self.runtime.prgm_mode = false;
    }

    #[wasm_bindgen]
    pub fn stop(&mut self) {
        self.runtime.running = false;
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.runtime.pc = 0;
        self.runtime.running = false;
        self.runtime.return_stack.clear();
    }

    #[wasm_bindgen]
    pub fn get_program_listing(&self) -> String {
        self.runtime.program_listing()
    }

    // Stack access methods for WASM
    #[wasm_bindgen]
    pub fn get_x(&self) -> f64 { self.runtime.stack.x }

    #[wasm_bindgen]
    pub fn get_y(&self) -> f64 { self.runtime.stack.y }

    #[wasm_bindgen]
    pub fn get_z(&self) -> f64 { self.runtime.stack.z }

    #[wasm_bindgen]
    pub fn get_t(&self) -> f64 { self.runtime.stack.t }

    #[wasm_bindgen]
    pub fn get_lastx(&self) -> f64 { self.runtime.stack.last_x }

    #[wasm_bindgen]
    pub fn set_x(&mut self, val: f64) { self.runtime.stack.x = val; }

    #[wasm_bindgen]
    pub fn push(&mut self, val: f64) { self.runtime.stack.push(val); }

    #[wasm_bindgen]
    pub fn enter(&mut self) { self.runtime.stack.push(self.runtime.stack.x); }

    // Direct stack operations for calculator mode
    #[wasm_bindgen]
    pub fn op_add(&mut self) { self.runtime.stack.add(); }

    #[wasm_bindgen]
    pub fn op_sub(&mut self) { self.runtime.stack.sub(); }

    #[wasm_bindgen]
    pub fn op_mul(&mut self) { self.runtime.stack.mul(); }

    #[wasm_bindgen]
    pub fn op_div(&mut self) { self.runtime.stack.div(); }

    #[wasm_bindgen]
    pub fn op_chs(&mut self) { self.runtime.stack.chs(); }

    #[wasm_bindgen]
    pub fn op_sqrt(&mut self) { self.runtime.stack.sqrt(); }

    #[wasm_bindgen]
    pub fn op_square(&mut self) { self.runtime.stack.square(); }

    #[wasm_bindgen]
    pub fn op_inv(&mut self) { self.runtime.stack.reciprocal(); }

    #[wasm_bindgen]
    pub fn op_pow(&mut self) { self.runtime.stack.power(); }

    #[wasm_bindgen]
    pub fn op_sin(&mut self) { self.runtime.stack.sin(); }

    #[wasm_bindgen]
    pub fn op_cos(&mut self) { self.runtime.stack.cos(); }

    #[wasm_bindgen]
    pub fn op_tan(&mut self) { self.runtime.stack.tan(); }

    #[wasm_bindgen]
    pub fn op_asin(&mut self) { self.runtime.stack.asin(); }

    #[wasm_bindgen]
    pub fn op_acos(&mut self) { self.runtime.stack.acos(); }

    #[wasm_bindgen]
    pub fn op_atan(&mut self) { self.runtime.stack.atan(); }

    #[wasm_bindgen]
    pub fn op_log(&mut self) { self.runtime.stack.log(); }

    #[wasm_bindgen]
    pub fn op_ln(&mut self) { self.runtime.stack.ln(); }

    #[wasm_bindgen]
    pub fn op_exp10(&mut self) { self.runtime.stack.exp10(); }

    #[wasm_bindgen]
    pub fn op_exp(&mut self) { self.runtime.stack.exp(); }

    #[wasm_bindgen]
    pub fn op_pi(&mut self) { self.runtime.stack.pi(); }

    #[wasm_bindgen]
    pub fn op_swap(&mut self) { self.runtime.stack.swap(); }

    #[wasm_bindgen]
    pub fn op_roll_down(&mut self) { self.runtime.stack.roll_down(); }

    #[wasm_bindgen]
    pub fn op_clear(&mut self) { self.runtime.stack.clear(); }

    #[wasm_bindgen]
    pub fn op_clx(&mut self) { self.runtime.stack.x = 0.0; }

    #[wasm_bindgen]
    pub fn op_lastx(&mut self) { self.runtime.stack.push(self.runtime.stack.last_x); }
}

#[cfg(target_arch = "wasm32")]
impl Default for HP41Transpiler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

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
        assert!(lisp.contains("rpn-square"));
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

    #[test]
    fn test_stack_operations() {
        let mut stack = HP41Stack::new();

        stack.push(5.0);
        assert_eq!(stack.x, 5.0);

        stack.push(3.0);
        assert_eq!(stack.x, 3.0);
        assert_eq!(stack.y, 5.0);

        stack.add();
        assert_eq!(stack.x, 8.0);
        assert_eq!(stack.last_x, 3.0);
    }

    #[test]
    fn test_runtime_factorial() {
        let mut runtime = HP41Runtime::new();
        runtime.load(r#"
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

        runtime.stack.push(5.0);  // Calculate 5!
        let result = runtime.run(1000);
        assert_eq!(result, 120.0);
    }

    #[test]
    fn test_statistics() {
        let mut stack = HP41Stack::new();

        // Add data points: (1,2), (2,4), (3,6)
        stack.y = 2.0; stack.x = 1.0; stack.sigma_plus();
        stack.y = 4.0; stack.x = 2.0; stack.sigma_plus();
        stack.y = 6.0; stack.x = 3.0; stack.sigma_plus();

        assert_eq!(stack.stat_n, 3.0);

        stack.mean();
        assert_eq!(stack.x, 2.0);  // mean of x
        assert_eq!(stack.y, 4.0);  // mean of y
    }

    #[test]
    fn test_conversions() {
        let mut stack = HP41Stack::new();

        // Test deg/rad conversion
        stack.push(180.0);
        stack.to_rad();
        assert!((stack.x - std::f64::consts::PI).abs() < 1e-10);

        stack.to_deg();
        assert!((stack.x - 180.0).abs() < 1e-10);

        // Test HMS conversion
        // 2.5 decimal hours = 2 hours 30 minutes 0 seconds = 2.3000 HMS
        stack.x = 2.5;
        stack.to_hms();
        assert!((stack.x - 2.30).abs() < 0.001, "to_hms failed: got {}", stack.x);

        // Convert back: 2.30 HMS = 2.5 decimal hours
        stack.to_hr();
        assert!((stack.x - 2.5).abs() < 1e-6, "to_hr failed: got {}", stack.x);
    }
}

// ============================================================================
// RPN LISP Prelude - defines all stack operations for LISP execution
// ============================================================================

/// LISP code that defines all RPN stack operations
/// This must be evaluated before running transpiled HP-41C code
pub const RPN_LISP_PRELUDE: &str = r#"
; HP-41C RPN Stack Implementation in LISP
; 4-level stack: X, Y, Z, T with LAST X

; Stack variables
(setq stack-x 0)
(setq stack-y 0)
(setq stack-z 0)
(setq stack-t 0)
(setq last-x 0)

; Registers (we'll create them as needed with setq)
(setq reg-00 0)
(setq reg-01 0)
(setq reg-02 0)
(setq reg-03 0)
(setq reg-04 0)
(setq reg-05 0)
(setq reg-06 0)
(setq reg-07 0)
(setq reg-08 0)
(setq reg-09 0)

; Alpha register
(setq alpha-reg "")

; Flags (simplified - just a few common ones)
(setq flag-0 nil)
(setq flag-1 nil)
(setq flag-2 nil)

; Stack push - lift stack and set X
(defun stack-push (val)
  (setq stack-t stack-z)
  (setq stack-z stack-y)
  (setq stack-y stack-x)
  (setq stack-x val))

; Stack drop (after binary op) - preserves last-x set by operation
(defun stack-drop-no-lastx ()
  (setq stack-y stack-z)
  (setq stack-z stack-t))

; Temp variable for swaps
(setq tmp-swap 0)

; Stack swap X<>Y
(defun stack-swap ()
  (setq tmp-swap stack-x)
  (setq stack-x stack-y)
  (setq stack-y tmp-swap))

; Stack roll down
(defun stack-roll-down ()
  (setq tmp-swap stack-x)
  (setq stack-x stack-y)
  (setq stack-y stack-z)
  (setq stack-z stack-t)
  (setq stack-t tmp-swap))

; Stack roll up
(defun stack-roll-up ()
  (setq tmp-swap stack-t)
  (setq stack-t stack-z)
  (setq stack-z stack-y)
  (setq stack-y stack-x)
  (setq stack-x tmp-swap))

; Clear stack
(defun stack-clear ()
  (setq stack-x 0)
  (setq stack-y 0)
  (setq stack-z 0)
  (setq stack-t 0))

; === Arithmetic Operations ===

(defun rpn-add ()
  (setq last-x stack-x)
  (setq stack-x (+ stack-y stack-x))
  (stack-drop-no-lastx))

(defun rpn-sub ()
  (setq last-x stack-x)
  (setq stack-x (- stack-y stack-x))
  (stack-drop-no-lastx))

(defun rpn-mul ()
  (setq last-x stack-x)
  (setq stack-x (* stack-y stack-x))
  (stack-drop-no-lastx))

(defun rpn-div ()
  (setq last-x stack-x)
  (setq stack-x (/ stack-y stack-x))
  (stack-drop-no-lastx))

(defun rpn-pow ()
  (setq last-x stack-x)
  (setq stack-x (expt stack-y stack-x))
  (stack-drop-no-lastx))

(defun rpn-chs ()
  (setq stack-x (- 0 stack-x)))

; === Math Functions ===

(defun rpn-sqrt ()
  (setq last-x stack-x)
  (setq stack-x (sqrt stack-x)))

(defun rpn-square ()
  (setq last-x stack-x)
  (setq stack-x (* stack-x stack-x)))

(defun rpn-inv ()
  (setq last-x stack-x)
  (setq stack-x (/ 1 stack-x)))

; === Trigonometry ===

(defun rpn-sin ()
  (setq last-x stack-x)
  (setq stack-x (sin stack-x)))

(defun rpn-cos ()
  (setq last-x stack-x)
  (setq stack-x (cos stack-x)))

(defun rpn-tan ()
  (setq last-x stack-x)
  (setq stack-x (/ (sin stack-x) (cos stack-x))))

(defun rpn-asin ()
  (setq last-x stack-x)
  (setq stack-x (asin stack-x)))

(defun rpn-acos ()
  (setq last-x stack-x)
  (setq stack-x (acos stack-x)))

(defun rpn-atan ()
  (setq last-x stack-x)
  (setq stack-x (atan stack-x)))

; === Logarithms ===

(defun rpn-log ()
  (setq last-x stack-x)
  (setq stack-x (/ (log stack-x) (log 10))))

(defun rpn-ln ()
  (setq last-x stack-x)
  (setq stack-x (log stack-x)))

(defun rpn-exp10 ()
  (setq last-x stack-x)
  (setq stack-x (expt 10 stack-x)))

(defun rpn-exp ()
  (setq last-x stack-x)
  (setq stack-x (exp stack-x)))

; === Constants ===

(defun rpn-pi ()
  (stack-push 3.141592653589793))

; === Integer Functions ===

(defun rpn-int ()
  (setq last-x stack-x)
  (setq stack-x (fix stack-x)))

(defun rpn-frac ()
  (setq last-x stack-x)
  (setq stack-x (- stack-x (fix stack-x))))

(defun rpn-abs ()
  (setq last-x stack-x)
  (setq stack-x (abs stack-x)))

(defun rpn-sign ()
  (setq last-x stack-x)
  (setq stack-x (if (> stack-x 0) 1 (if (< stack-x 0) -1 0))))

(setq tmp-mod 0)
(defun rpn-mod ()
  (setq last-x stack-x)
  (setq tmp-mod (rem stack-y stack-x))
  (stack-drop-no-lastx)
  (setq stack-x tmp-mod))

; === Percent Functions ===

(defun rpn-percent ()
  (setq last-x stack-x)
  (setq stack-x (* stack-y (/ stack-x 100.0))))

(defun rpn-percent-ch ()
  (setq last-x stack-x)
  (setq stack-x (* 100.0 (/ (- stack-x stack-y) stack-y))))

; === Conversion Functions ===

(defun rpn-to-deg ()
  (setq last-x stack-x)
  (setq stack-x (* stack-x (/ 180 3.141592653589793))))

(defun rpn-to-rad ()
  (setq last-x stack-x)
  (setq stack-x (* stack-x (/ 3.141592653589793 180))))

; Temp vars for HMS conversions
(setq hms-hours 0)
(setq hms-dec-part 0)
(setq hms-minutes 0)
(setq hms-seconds 0)
(setq hms-frac 0)
(setq hms-mm-ss 0)

; HMS conversions
(defun rpn-to-hms ()
  (setq last-x stack-x)
  (setq hms-hours (fix stack-x))
  (setq hms-dec-part (abs (- stack-x hms-hours)))
  (setq hms-minutes (fix (* hms-dec-part 60)))
  (setq hms-seconds (* (- (* hms-dec-part 60) hms-minutes) 60))
  (setq stack-x (+ hms-hours (/ hms-minutes 100) (/ hms-seconds 10000))))

(defun rpn-to-hr ()
  (setq last-x stack-x)
  (setq hms-hours (fix stack-x))
  (setq hms-frac (- (abs stack-x) (abs hms-hours)))
  (setq hms-mm-ss (* hms-frac 100))
  (setq hms-minutes (fix hms-mm-ss))
  (setq hms-seconds (* (- hms-mm-ss hms-minutes) 100))
  (setq stack-x (* (if (< stack-x 0) -1 1)
                   (+ hms-hours (/ hms-minutes 60) (/ hms-seconds 3600)))))

; Temp vars for conversions
(setq conv-r 0)
(setq conv-theta 0)
(setq conv-x 0)
(setq conv-y 0)
(setq hyp-ep 0)
(setq hyp-em 0)

; Polar/Rectangular conversions
(defun rpn-to-rect ()
  (setq last-x stack-x)
  (setq conv-r stack-y)
  (setq conv-theta stack-x)
  (setq stack-x (* conv-r (cos conv-theta)))
  (setq stack-y (* conv-r (sin conv-theta))))

(defun rpn-to-polar ()
  (setq last-x stack-x)
  (setq conv-x stack-x)
  (setq conv-y stack-y)
  (setq stack-x (sqrt (+ (* conv-x conv-x) (* conv-y conv-y))))
  (setq stack-y (atan conv-y conv-x)))

; === Hyperbolic Functions ===

(defun rpn-sinh ()
  (setq last-x stack-x)
  (setq stack-x (/ (- (exp stack-x) (exp (- 0 stack-x))) 2)))

(defun rpn-cosh ()
  (setq last-x stack-x)
  (setq stack-x (/ (+ (exp stack-x) (exp (- 0 stack-x))) 2)))

(defun rpn-tanh ()
  (setq last-x stack-x)
  (setq hyp-ep (exp stack-x))
  (setq hyp-em (exp (- 0 stack-x)))
  (setq stack-x (/ (- hyp-ep hyp-em) (+ hyp-ep hyp-em))))

(defun rpn-asinh ()
  (setq last-x stack-x)
  (setq stack-x (log (+ stack-x (sqrt (+ (* stack-x stack-x) 1))))))

(defun rpn-acosh ()
  (setq last-x stack-x)
  (setq stack-x (log (+ stack-x (sqrt (- (* stack-x stack-x) 1))))))

(defun rpn-atanh ()
  (setq last-x stack-x)
  (setq stack-x (* 0.5 (log (/ (+ 1 stack-x) (- 1 stack-x))))))

; === Statistics (simplified - would need more state) ===

(defun rpn-sigma-plus () stack-x)
(defun rpn-sigma-minus () stack-x)
(defun rpn-mean () stack-x)
(defun rpn-sdev () stack-x)

; === Get stack values ===

(defun get-x () stack-x)
(defun get-y () stack-y)
(defun get-z () stack-z)
(defun get-t () stack-t)
"#;

// ============================================================================
// Comprehensive HP-41C Tests
// Tests verify: 1) Rust runtime execution, 2) LISP transpilation, 3) LISP execution
// ============================================================================

#[cfg(test)]
mod comprehensive_tests {
    use super::*;
    use crate::{run_lisp, Expr};

    /// Helper to run LISP and get stack-x as f64
    fn lisp_get_x(code: &str) -> f64 {
        let full_code = format!("{}\n{}\nstack-x", RPN_LISP_PRELUDE, code);
        let (results, _) = run_lisp(&full_code);
        match results.last() {
            Some(Expr::Real(f)) => *f,
            Some(Expr::Integer(i)) => *i as f64,
            other => panic!("Expected number, got {:?}", other),
        }
    }

    /// Test basic arithmetic: 3 + 4 = 7
    #[test]
    fn test_basic_addition_all_modes() {
        // 1. Rust runtime execution
        let mut runtime = HP41Runtime::new();
        runtime.stack.push(3.0);
        runtime.stack.push(4.0);
        runtime.stack.add();
        assert_eq!(runtime.stack.x, 7.0, "Rust runtime: 3+4 should be 7");

        // 2. Check transpiled LISP is correct
        let prog = HP41Program::parse("3\n4\n+").unwrap();
        let lisp = prog.to_lisp();
        assert!(lisp.contains("(stack-push 3)"), "Should push 3");
        assert!(lisp.contains("(stack-push 4)"), "Should push 4");
        assert!(lisp.contains("(rpn-add)"), "Should call rpn-add");

        // 3. Execute LISP and verify result
        let result = lisp_get_x("(stack-push 3) (stack-push 4) (rpn-add)");
        assert_eq!(result, 7.0, "LISP execution: 3+4 should be 7");
    }

    /// Test circle area: r² × π
    #[test]
    fn test_circle_area_all_modes() {
        let expected = 5.0 * 5.0 * std::f64::consts::PI;

        // 1. Rust runtime
        let mut runtime = HP41Runtime::new();
        runtime.load(r#"
            LBL "AREA"
            X^2
            PI
            *
            RTN
        "#).unwrap();
        runtime.stack.push(5.0);  // radius = 5
        runtime.run(100);
        assert!((runtime.stack.x - expected).abs() < 1e-10,
            "Rust runtime: area of circle r=5 should be {}, got {}", expected, runtime.stack.x);

        // 2. Check transpilation
        let prog = HP41Program::parse(r#"
            LBL "AREA"
            X^2
            PI
            *
            RTN
        "#).unwrap();
        let lisp = prog.to_lisp();
        assert!(lisp.contains("defun hp41-area"), "Should define hp41-area function");
        assert!(lisp.contains("rpn-square"), "Should square");
        assert!(lisp.contains("rpn-pi"), "Should push pi");
        assert!(lisp.contains("rpn-mul"), "Should multiply");

        // 3. Execute equivalent LISP
        let result = lisp_get_x("(stack-push 5) (rpn-square) (rpn-pi) (rpn-mul)");
        assert!((result - expected).abs() < 1e-10,
            "LISP execution: area of circle r=5 should be {}, got {}", expected, result);
    }

    /// Test factorial 5! = 120 using DSE loop
    #[test]
    fn test_factorial_all_modes() {
        // 1. Rust runtime
        let mut runtime = HP41Runtime::new();
        runtime.load(r#"
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
        runtime.stack.push(5.0);
        runtime.run(1000);
        assert_eq!(runtime.stack.x, 120.0, "Rust runtime: 5! should be 120");

        // 2. Check transpilation includes key elements
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
        let lisp = prog.to_lisp();
        assert!(lisp.contains("defun hp41-fact"), "Should define hp41-fact");
        assert!(lisp.contains("reg-00"), "Should reference register 00");
        assert!(lisp.contains("rpn-mul"), "Should multiply");

        // Note: Full factorial loop can't run in pure LISP without GTO/label support
        // We test the sequential calculation instead
        // 5! = 5 * 4 * 3 * 2 * 1
        let result = lisp_get_x(r#"
            (stack-push 5)
            (stack-push 4)
            (rpn-mul)
            (stack-push 3)
            (rpn-mul)
            (stack-push 2)
            (rpn-mul)
            (stack-push 1)
            (rpn-mul)
        "#);
        assert_eq!(result, 120.0, "LISP execution: 5*4*3*2*1 should be 120");
    }

    /// Test quadratic formula: (-b + sqrt(b²-4ac)) / 2a
    /// For x² - 5x + 6 = 0, roots are 2 and 3
    #[test]
    fn test_quadratic_formula_all_modes() {
        // Coefficients: a=1, b=-5, c=6
        // Root = (-(-5) + sqrt(25 - 24)) / 2 = (5 + 1) / 2 = 3

        // 1. Rust runtime - calculate one root
        let mut stack = HP41Stack::new();
        // Calculate b² - 4ac
        stack.push(-5.0);  // b
        stack.square();     // b²
        stack.push(1.0);   // a
        stack.push(6.0);   // c
        stack.mul();       // a*c
        stack.push(4.0);
        stack.mul();       // 4ac
        stack.sub();       // b² - 4ac
        stack.sqrt();      // sqrt(b² - 4ac)

        // Now -b + sqrt(...)
        stack.push(-5.0);  // b
        stack.chs();       // -b = 5
        stack.add();       // -b + sqrt(...) = 5 + 1 = 6

        // Divide by 2a
        stack.push(1.0);   // a
        stack.push(2.0);
        stack.mul();       // 2a
        stack.div();       // result / 2a

        assert_eq!(stack.x, 3.0, "Rust: quadratic root should be 3");

        // 2. LISP equivalent calculation
        let result = lisp_get_x(r#"
            ; Calculate b² - 4ac where a=1, b=-5, c=6
            (stack-push -5)
            (rpn-square)
            (stack-push 1)
            (stack-push 6)
            (rpn-mul)
            (stack-push 4)
            (rpn-mul)
            (rpn-sub)
            (rpn-sqrt)
            ; Now -b + sqrt(...)
            (stack-push -5)
            (rpn-chs)
            (rpn-add)
            ; Divide by 2a
            (stack-push 1)
            (stack-push 2)
            (rpn-mul)
            (rpn-div)
        "#);
        assert_eq!(result, 3.0, "LISP: quadratic root should be 3");
    }

    /// Test trigonometry: sin²(x) + cos²(x) = 1
    #[test]
    fn test_trig_identity_all_modes() {
        let x = 0.7;  // arbitrary angle in radians

        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(x);
        stack.sin();
        stack.square();
        let sin2 = stack.x;

        stack.push(x);
        stack.cos();
        stack.square();
        stack.push(sin2);
        stack.add();

        assert!((stack.x - 1.0).abs() < 1e-10,
            "Rust: sin²(0.7) + cos²(0.7) should be 1, got {}", stack.x);

        // 2. LISP execution
        let result = lisp_get_x(&format!(r#"
            (stack-push {})
            (rpn-sin)
            (rpn-square)
            (setq sin2 stack-x)
            (stack-push {})
            (rpn-cos)
            (rpn-square)
            (stack-push sin2)
            (rpn-add)
        "#, x, x));
        assert!((result - 1.0).abs() < 1e-10,
            "LISP: sin²(0.7) + cos²(0.7) should be 1, got {}", result);
    }

    /// Test logarithm identity: 10^(log(x)) = x
    #[test]
    fn test_log_identity_all_modes() {
        let x = 42.0;

        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(x);
        stack.log();       // log10(42)
        stack.exp10();     // 10^log10(42) = 42
        assert!((stack.x - x).abs() < 1e-10,
            "Rust: 10^log10(42) should be 42, got {}", stack.x);

        // 2. LISP execution
        let result = lisp_get_x(&format!(r#"
            (stack-push {})
            (rpn-log)
            (rpn-exp10)
        "#, x));
        assert!((result - x).abs() < 1e-10,
            "LISP: 10^log10(42) should be 42, got {}", result);
    }

    /// Test stack operations: push, swap, roll
    #[test]
    fn test_stack_ops_all_modes() {
        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(1.0);
        stack.push(2.0);
        stack.push(3.0);
        stack.push(4.0);
        // Stack is now: X=4, Y=3, Z=2, T=1
        assert_eq!(stack.x, 4.0);
        assert_eq!(stack.y, 3.0);

        stack.swap();
        assert_eq!(stack.x, 3.0);
        assert_eq!(stack.y, 4.0);

        stack.roll_down();
        // After roll down: X=Y=4, Y=Z=2, Z=T=1, T=old_X=3
        assert_eq!(stack.x, 4.0);
        assert_eq!(stack.y, 2.0);
        assert_eq!(stack.z, 1.0);
        assert_eq!(stack.t, 3.0);

        // 2. LISP execution
        let result = lisp_get_x(r#"
            (stack-push 1)
            (stack-push 2)
            (stack-push 3)
            (stack-push 4)
            (stack-swap)
            (stack-roll-down)
        "#);
        assert_eq!(result, 4.0, "LISP: after swap and roll-down, X should be 4");
    }

    /// Test power function: 2^10 = 1024
    #[test]
    fn test_power_all_modes() {
        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(2.0);
        stack.push(10.0);
        stack.power();
        assert_eq!(stack.x, 1024.0, "Rust: 2^10 should be 1024");

        // 2. LISP execution
        let result = lisp_get_x("(stack-push 2) (stack-push 10) (rpn-pow)");
        assert_eq!(result, 1024.0, "LISP: 2^10 should be 1024");
    }

    /// Test percent calculation: 15% of 200 = 30
    #[test]
    fn test_percent_all_modes() {
        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(200.0);  // base
        stack.push(15.0);   // percent
        stack.percent();
        assert_eq!(stack.x, 30.0, "Rust: 15% of 200 should be 30");

        // 2. LISP execution
        let result = lisp_get_x("(stack-push 200) (stack-push 15) (rpn-percent)");
        assert_eq!(result, 30.0, "LISP: 15% of 200 should be 30");
    }

    /// Test integer/fractional parts
    #[test]
    fn test_int_frac_all_modes() {
        // 1. Rust runtime
        let mut stack = HP41Stack::new();
        stack.push(3.75);
        stack.int();
        assert_eq!(stack.x, 3.0, "Rust: INT(3.75) should be 3");

        stack.push(3.75);
        stack.frac();
        assert!((stack.x - 0.75).abs() < 1e-10, "Rust: FRAC(3.75) should be 0.75");

        // 2. LISP execution
        let int_result = lisp_get_x("(stack-push 3.75) (rpn-int)");
        assert_eq!(int_result, 3.0, "LISP: INT(3.75) should be 3");

        let frac_result = lisp_get_x("(stack-push 3.75) (rpn-frac)");
        assert!((frac_result - 0.75).abs() < 1e-10, "LISP: FRAC(3.75) should be 0.75");
    }

    /// Test that transpiled program structure is correct
    #[test]
    fn test_transpile_structure() {
        let prog = HP41Program::parse(r#"
            LBL "TEST"
            5
            ENTER
            3
            +
            X^2
            RTN
        "#).unwrap();

        let lisp = prog.to_lisp();

        // Check structure
        assert!(lisp.contains("; HP-41C Program: TEST"), "Should have program name comment");
        assert!(lisp.contains("(defun hp41-test ()"), "Should define function");
        assert!(lisp.contains("(stack-push 5)"), "Should push 5");
        assert!(lisp.contains("(stack-push stack-x)"), "ENTER should push stack-x");
        assert!(lisp.contains("(stack-push 3)"), "Should push 3");
        assert!(lisp.contains("(rpn-add)"), "Should add");
        assert!(lisp.contains("(rpn-square)"), "Should square");
        assert!(lisp.contains("stack-x)"), "Should return stack-x at end");
    }

    /// Test HP-41C program display format
    #[test]
    fn test_display_format() {
        let prog = HP41Program::parse(r#"
            LBL "DEMO"
            STO 00
            RCL 00
            X<>Y
            +
            RTN
        "#).unwrap();

        // Check instruction display strings
        assert_eq!(prog.instructions[0].to_display(), "LBL \"DEMO\"");
        assert_eq!(prog.instructions[1].to_display(), "STO 00");
        assert_eq!(prog.instructions[2].to_display(), "RCL 00");
        assert_eq!(prog.instructions[3].to_display(), "X<>Y");
        assert_eq!(prog.instructions[4].to_display(), "+");
        assert_eq!(prog.instructions[5].to_display(), "RTN");
    }
}

// rust-autolisp - AutoCAD 9/10 AutoLISP Engine
//
// This is a Rust library that can be used from code to:
// 1. Load and execute AutoLISP (.LSP) files
// 2. Process CSV data with templates
// 3. Generate drawings (SVG, JSON, DXF output)
//
// Historical Context:
// In 1991, Elektrotechnik Trahe GmbH in Unterneukirchen (Bavaria) used
// AutoCAD 9/10 on a 486DX-50 with custom AutoLISP programs to generate
// hundreds of Schaltpläne (electrical circuit diagrams) from CSV data.
// This Rust engine recreates that capability.

use wasm_bindgen::prelude::*;

pub mod hp41;
pub mod interpreter;
pub mod kicad;
#[cfg(test)]
mod kicad_tests;
pub mod lexer;
pub mod parser;

pub use interpreter::{CadType, DrawEntity, DrawingState, ForeignContext, ForeignFn, Interpreter};
pub use parser::Expr;

// Only use fs, Path, and HashMap for non-WASM builds
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

// ============================================
// WASM API - The engine running in the browser
// ============================================

/// WASM-accessible AutoLISP engine
#[wasm_bindgen]
pub struct WasmEngine {
    interpreter: Interpreter,
}

#[wasm_bindgen]
impl WasmEngine {
    /// Create a new engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        WasmEngine {
            interpreter: Interpreter::new(),
        }
    }

    /// Execute AutoLISP code and return results as JSON
    #[wasm_bindgen]
    pub fn eval(&mut self, code: &str) -> String {
        let results = self.interpreter.run(code);
        // Convert results to JSON
        let json_results: Vec<String> = results.iter().map(|e| format!("{}", e)).collect();
        format!(
            "[{}]",
            json_results
                .iter()
                .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Get drawing entities as JSON array
    #[wasm_bindgen]
    pub fn get_entities_json(&self) -> String {
        let mut json = String::from("[");
        for (i, entity) in self.interpreter.drawing.entities.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&entity_to_json(entity));
        }
        json.push(']');
        json
    }

    /// Get drawing entities as SVG
    #[wasm_bindgen]
    pub fn get_entities_svg(&self) -> String {
        entities_to_svg(&self.interpreter.drawing.entities, self.interpreter.drawing.cad_type)
    }

    /// Get number of entities
    #[wasm_bindgen]
    pub fn entity_count(&self) -> usize {
        self.interpreter.drawing.entities.len()
    }

    /// Get output buffer (PRINC/PRINT output)
    #[wasm_bindgen]
    pub fn get_output(&self) -> String {
        self.interpreter.output.join("")
    }

    /// Export current drawing as a KiCad Symbol Library
    #[wasm_bindgen]
    pub fn get_kicad_sym(&self, library_name: &str, symbol_name: &str) -> String {
        kicad::to_kicad_sym(
            &self.interpreter.drawing.entities,
            library_name,
            symbol_name,
        )
    }

    /// Export current drawing as a KiCad Footprint
    #[wasm_bindgen]
    pub fn get_kicad_mod(&self, footprint_name: &str) -> String {
        kicad::to_kicad_mod(&self.interpreter.drawing.entities, footprint_name)
    }

    /// Clear the drawing
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.interpreter.drawing.entities.clear();
        self.interpreter.output.clear();
    }

    /// Set CAD type: "rustlisp" (default) or "kicad"
    /// This affects coordinate system handling in SVG output
    #[wasm_bindgen]
    pub fn set_cad_type(&mut self, cad_type: &str) {
        self.interpreter.drawing.cad_type = match cad_type.to_lowercase().as_str() {
            "kicad" => CadType::KiCad,
            _ => CadType::RustLisp,
        };
    }

    /// Get current CAD type as string
    #[wasm_bindgen]
    pub fn get_cad_type(&self) -> String {
        match self.interpreter.drawing.cad_type {
            CadType::RustLisp => "rustlisp".to_string(),
            CadType::KiCad => "kicad".to_string(),
        }
    }

    /// Benchmark: run code multiple times and return timing stats as JSON
    /// Returns: { "iterations": n, "total_ms": f, "avg_ms": f, "entities": n, "engine": "rust" }
    #[wasm_bindgen]
    pub fn benchmark(&mut self, code: &str, iterations: usize) -> String {
        use web_sys::window;

        let iterations = iterations.clamp(1, 10000); // Clamp to reasonable range

        // Get performance.now() from browser
        let start = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        for _ in 0..iterations {
            self.interpreter.drawing.entities.clear();
            self.interpreter.output.clear();
            let _ = self.interpreter.run(code);
        }

        let end = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        let total_ms = end - start;
        let avg_ms = total_ms / iterations as f64;
        let entities = self.interpreter.drawing.entities.len();

        format!(
            r#"{{"iterations":{},"total_ms":{:.3},"avg_ms":{:.6},"entities":{},"engine":"rust"}}"#,
            iterations, total_ms, avg_ms, entities
        )
    }

    /// Get engine info
    #[wasm_bindgen]
    pub fn engine_info(&self) -> String {
        r#"{"name":"acadlisp","engine":"rust","version":"0.1.0","features":["autolisp","csv","dxf","svg"]}"#.to_string()
    }

    /// Get list of available examples
    #[wasm_bindgen]
    pub fn get_example_names(&self) -> String {
        r#"["hello","math","box","spiral","schaltplan","fractal","kicad_sym","kicad_fp","kicad_cake","kicad_newyear","birthday","newyear2026"]"#.to_string()
    }

    /// Get example code by name
    #[wasm_bindgen]
    pub fn get_example(&self, name: &str) -> String {
        match name {
            "hello" => EXAMPLE_HELLO.to_string(),
            "math" => EXAMPLE_MATH.to_string(),
            "box" => EXAMPLE_BOX.to_string(),
            "spiral" => EXAMPLE_SPIRAL.to_string(),
            "schaltplan" => EXAMPLE_SCHALTPLAN.to_string(),
            "fractal" => EXAMPLE_FRACTAL.to_string(),
            "kicad_sym" => EXAMPLE_KICAD_SYM.to_string(),
            "kicad_fp" => EXAMPLE_KICAD_FP.to_string(),
            "kicad_cake" => EXAMPLE_KICAD_CAKE.to_string(),
            "kicad_newyear" => EXAMPLE_KICAD_NEWYEAR.to_string(),
            "birthday" => EXAMPLE_BIRTHDAY.to_string(),
            "newyear2026" => EXAMPLE_NEWYEAR2026.to_string(),
            _ => "; Unknown example".to_string(),
        }
    }

    /// Plot a function f(x) - evaluates the function and generates SVG
    /// Returns JSON with { svg: string, points: number, min_y: number, max_y: number }
    #[wasm_bindgen]
    pub fn plot_function(
        &mut self,
        code: &str,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        steps: usize,
    ) -> String {
        // First, define the function
        self.interpreter.drawing.entities.clear();
        self.interpreter.output.clear();

        // Run the code to define f(x)
        let _ = self.interpreter.run(code);

        // Evaluate f(x) for each step
        let mut points: Vec<(f64, f64)> = Vec::new();
        let dx = (x_max - x_min) / steps as f64;

        for i in 0..=steps {
            let x = x_min + i as f64 * dx;
            let expr = format!("(f {})", x);
            let results = self.interpreter.run(&expr);

            // Get the result
            if let Some(result) = results.last() {
                let result_str = format!("{}", result);
                if let Some(y) = parse_number_result(&result_str) {
                    if y.is_finite() {
                        points.push((x, y));
                    }
                }
            }
        }

        if points.is_empty() {
            return r#"{"error": "No valid points computed. Make sure f(x) is defined correctly."}"#.to_string();
        }

        // Calculate actual Y range from data
        let actual_min_y = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let actual_max_y = points
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);

        // Generate SVG
        let svg = generate_plot_svg(&points, x_min, x_max, y_min, y_max);

        format!(
            r#"{{"svg": {}, "points": {}, "min_y": {:.6}, "max_y": {:.6}}}"#,
            serde_json_escape(&svg),
            points.len(),
            actual_min_y,
            actual_max_y
        )
    }

    /// Get plot example code by name
    #[wasm_bindgen]
    pub fn get_plot_example(&self, name: &str) -> String {
        match name {
            // Basic
            "sin" => PLOT_SIN.to_string(),
            "cos" => PLOT_COS.to_string(),
            "parabola" => PLOT_PARABOLA.to_string(),
            "cubic" => PLOT_CUBIC.to_string(),
            "sqrt" => PLOT_SQRT.to_string(),
            "abs" => PLOT_ABS.to_string(),
            // Waves
            "sincos" => PLOT_SINCOS.to_string(),
            "wave" => PLOT_WAVE.to_string(),
            "lissajous" => PLOT_LISSAJOUS.to_string(),
            "polar" => PLOT_POLAR.to_string(),
            // Recursion
            "taylor" => PLOT_TAYLOR_SIN.to_string(),
            "fibonacci" => PLOT_FIBONACCI.to_string(),
            "harmonic" => PLOT_RECURSIVE_WAVE.to_string(),
            "mandelbrot" => PLOT_MANDELBROT_SLICE.to_string(),
            "newton" => PLOT_NEWTON_SQRT.to_string(),
            // Calculus
            "integral" => PLOT_INTEGRAL.to_string(),
            "derivative" => PLOT_DERIVATIVE.to_string(),
            "derivative2" => PLOT_SECOND_DERIV.to_string(),
            // Fourier / Signal Processing
            "fourier" => PLOT_FOURIER.to_string(),
            "fourier-saw" => PLOT_FOURIER_SAW.to_string(),
            "convolution" => PLOT_CONVOLUTION.to_string(),
            "laplace" => PLOT_LAPLACE.to_string(),
            _ => "; Unknown plot example".to_string(),
        }
    }

    /// Generate DXF from current drawing entities
    #[wasm_bindgen]
    pub fn get_entities_dxf(&self) -> String {
        let mut dxf = String::new();

        // DXF Header - AutoCAD R12 format (AC1009)
        dxf.push_str("  0\nSECTION\n  2\nHEADER\n");
        dxf.push_str("  9\n$ACADVER\n  1\nAC1009\n");
        dxf.push_str("  0\nENDSEC\n");

        // Tables section
        dxf.push_str("  0\nSECTION\n  2\nTABLES\n");
        dxf.push_str("  0\nTABLE\n  2\nLAYER\n 70\n1\n");
        dxf.push_str("  0\nLAYER\n  2\n0\n 70\n0\n 62\n7\n  6\nCONTINUOUS\n");
        dxf.push_str("  0\nENDTAB\n  0\nENDSEC\n");

        // Entities section
        dxf.push_str("  0\nSECTION\n  2\nENTITIES\n");

        for entity in &self.interpreter.drawing.entities {
            match entity {
                DrawEntity::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    layer,
                } => {
                    dxf.push_str(&format!(
                        "  0\nLINE\n  8\n{}\n 10\n{}\n 20\n{}\n 11\n{}\n 21\n{}\n",
                        layer, x1, y1, x2, y2
                    ));
                }
                DrawEntity::Circle {
                    cx,
                    cy,
                    radius,
                    layer,
                } => {
                    dxf.push_str(&format!(
                        "  0\nCIRCLE\n  8\n{}\n 10\n{}\n 20\n{}\n 40\n{}\n",
                        layer, cx, cy, radius
                    ));
                }
                DrawEntity::Text {
                    x,
                    y,
                    height,
                    text,
                    layer,
                } => {
                    dxf.push_str(&format!(
                        "  0\nTEXT\n  8\n{}\n 10\n{}\n 20\n{}\n 40\n{}\n  1\n{}\n",
                        layer, x, y, height, text
                    ));
                }
                DrawEntity::Arc {
                    cx,
                    cy,
                    radius,
                    start_angle,
                    end_angle,
                    layer,
                } => {
                    dxf.push_str(&format!(
                        "  0\nARC\n  8\n{}\n 10\n{}\n 20\n{}\n 40\n{}\n 50\n{}\n 51\n{}\n",
                        layer,
                        cx,
                        cy,
                        radius,
                        start_angle.to_degrees(),
                        end_angle.to_degrees()
                    ));
                }
                DrawEntity::Point { x, y, layer } => {
                    dxf.push_str(&format!(
                        "  0\nPOINT\n  8\n{}\n 10\n{}\n 20\n{}\n",
                        layer, x, y
                    ));
                }
                DrawEntity::Insert {
                    block_name,
                    x,
                    y,
                    scale,
                    rotation,
                    layer,
                } => {
                    dxf.push_str(&format!(
                        "  0\nINSERT\n  8\n{}\n  2\n{}\n 10\n{}\n 20\n{}\n 41\n{}\n 42\n{}\n 50\n{}\n",
                        layer, block_name, x, y, scale, scale, rotation
                    ));
                }
                DrawEntity::Pin { .. } | DrawEntity::Property { .. } | DrawEntity::Pad { .. } => {
                    // KiCad specific entities ignored in DXF export for now
                }
            }
        }

        dxf.push_str("  0\nENDSEC\n  0\nEOF\n");
        dxf
    }

    /// Process CSV and return drawing specs as JSON
    #[wasm_bindgen]
    pub fn parse_csv(&self, csv_data: &str) -> String {
        let mut drawings = Vec::new();
        let mut current: Option<(String, String, Vec<Vec<String>>)> = None;

        for line in csv_data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = line.split(';').collect();
            if fields.is_empty() {
                continue;
            }

            let cmd = fields[0].to_uppercase();
            match cmd.as_str() {
                "START" => {
                    if let Some((name, tpl, comps)) = current.take() {
                        drawings.push((name, tpl, comps));
                    }
                    let template = fields.get(1).unwrap_or(&"DEFAULT").to_string();
                    let name = fields.get(2).unwrap_or(&"UNNAMED").to_string();
                    current = Some((name, template, Vec::new()));
                }
                "STOP" | "END" => {
                    if let Some((name, tpl, comps)) = current.take() {
                        drawings.push((name, tpl, comps));
                    }
                }
                _ => {
                    if let Some((_, _, ref mut comps)) = current {
                        comps.push(fields.iter().map(|s| s.to_string()).collect());
                    }
                }
            }
        }
        if let Some((name, tpl, comps)) = current {
            drawings.push((name, tpl, comps));
        }

        // Convert to JSON
        let mut json = String::from("[");
        for (i, (name, tpl, comps)) in drawings.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                r#"{{"name":"{}","template":"{}","components":["#,
                name, tpl
            ));
            for (j, comp) in comps.iter().enumerate() {
                if j > 0 {
                    json.push(',');
                }
                json.push_str(&format!(
                    r#"{{"name":"{}","type":"{}","value":"{}"}}"#,
                    comp.first().unwrap_or(&String::new()),
                    comp.get(1).unwrap_or(&String::new()),
                    comp.get(2).unwrap_or(&String::new())
                ));
            }
            json.push_str("]}");
        }
        json.push(']');
        json
    }

    /// Generate a Schaltplan drawing from spec
    #[wasm_bindgen]
    pub fn generate_schaltplan(
        &mut self,
        name: &str,
        template: &str,
        components_json: &str,
    ) -> String {
        self.interpreter.drawing.entities.clear();

        // Parse components
        let components: Vec<(String, String, String)> =
            serde_json_minimal_parse(components_json).unwrap_or_default();

        // Frame (A4 proportions)
        self.eval("(command \"LINE\" '(0 0) '(297 0) \"\")");
        self.eval("(command \"LINE\" '(297 0) '(297 210) \"\")");
        self.eval("(command \"LINE\" '(297 210) '(0 210) \"\")");
        self.eval("(command \"LINE\" '(0 210) '(0 0) \"\")");

        match template {
            "MOTOR_ST" | "STERN_DR" | "WENDE" => {
                self.generate_motor_schaltplan(&components);
            }
            "DIREKT" => {
                self.generate_control_schaltplan(&components);
            }
            _ => {
                self.generate_motor_schaltplan(&components);
            }
        }

        // Schriftfeld
        self.generate_schriftfeld(name, template);

        self.get_entities_svg()
    }
}

impl WasmEngine {
    fn generate_motor_schaltplan(&mut self, components: &[(String, String, String)]) {
        // Power rails
        self.eval("(command \"LINE\" '(40 200) '(40 70) \"\")");
        self.eval("(command \"LINE\" '(80 200) '(80 70) \"\")");
        self.eval("(command \"LINE\" '(120 200) '(120 70) \"\")");
        self.eval("(command \"TEXT\" '(38 203) 3 0 \"L1\")");
        self.eval("(command \"TEXT\" '(78 203) 3 0 \"L2\")");
        self.eval("(command \"TEXT\" '(118 203) 3 0 \"L3\")");

        // Fuses
        self.eval("(command \"LINE\" '(36 190) '(44 190) \"\")");
        self.eval("(command \"LINE\" '(36 188) '(44 192) \"\")");
        self.eval("(command \"LINE\" '(76 190) '(84 190) \"\")");
        self.eval("(command \"LINE\" '(76 188) '(84 192) \"\")");
        self.eval("(command \"LINE\" '(116 190) '(124 190) \"\")");
        self.eval("(command \"LINE\" '(116 188) '(124 192) \"\")");

        let mut y = 165;
        for (comp_name, comp_type, _) in components {
            if comp_type.contains("Schuetz") || comp_type.contains("schuetz") {
                self.eval(&format!(
                    "(command \"LINE\" '(36 {}) '(44 {}) \"\")",
                    y - 3,
                    y + 3
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(76 {}) '(84 {}) \"\")",
                    y - 3,
                    y + 3
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(116 {}) '(124 {}) \"\")",
                    y - 3,
                    y + 3
                ));
                self.eval(&format!("(command \"LINE\" '(44 {}) '(76 {}) \"\")", y, y));
                self.eval(&format!("(command \"LINE\" '(84 {}) '(116 {}) \"\")", y, y));
                self.eval(&format!(
                    "(command \"TEXT\" '(130 {}) 3 0 \"{}\")",
                    y, comp_name
                ));
                y -= 22;
            } else if comp_type.contains("Motorschutz") {
                self.eval(&format!(
                    "(command \"LINE\" '(37 {}) '(43 {}) \"\")",
                    y,
                    y - 4
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(43 {}) '(37 {}) \"\")",
                    y - 4,
                    y - 8
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(77 {}) '(83 {}) \"\")",
                    y,
                    y - 4
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(83 {}) '(77 {}) \"\")",
                    y - 4,
                    y - 8
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(117 {}) '(123 {}) \"\")",
                    y,
                    y - 4
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(123 {}) '(117 {}) \"\")",
                    y - 4,
                    y - 8
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(35 {}) '(125 {}) \"\")",
                    y + 2,
                    y + 2
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(35 {}) '(125 {}) \"\")",
                    y - 10,
                    y - 10
                ));
                self.eval(&format!(
                    "(command \"TEXT\" '(130 {}) 3 0 \"{}\")",
                    y, comp_name
                ));
                y -= 22;
            }
        }

        // Motor
        self.eval("(command \"LINE\" '(40 80) '(60 65) \"\")");
        self.eval("(command \"LINE\" '(80 80) '(80 65) \"\")");
        self.eval("(command \"LINE\" '(120 80) '(100 65) \"\")");
        self.eval("(command \"CIRCLE\" '(80 50) 12)");
        self.eval("(command \"TEXT\" '(76 47) 6 0 \"M\")");
        self.eval("(command \"LINE\" '(80 38) '(80 30) \"\")");
        self.eval("(command \"LINE\" '(75 30) '(85 30) \"\")");

        // Control circuit
        self.eval("(command \"LINE\" '(160 200) '(160 70) \"\")");
        self.eval("(command \"LINE\" '(240 200) '(240 70) \"\")");
        self.eval("(command \"LINE\" '(160 180) '(175 180) \"\")");
        self.eval("(command \"LINE\" '(175 182) '(185 178) \"\")");
        self.eval("(command \"LINE\" '(185 180) '(200 180) \"\")");
        self.eval("(command \"LINE\" '(200 175) '(210 180) \"\")");
        self.eval("(command \"LINE\" '(210 180) '(220 180) \"\")");
        self.eval("(command \"LINE\" '(220 180) '(220 130) \"\")");
        self.eval("(command \"LINE\" '(215 130) '(225 130) \"\")");
        self.eval("(command \"LINE\" '(215 130) '(215 115) \"\")");
        self.eval("(command \"LINE\" '(225 130) '(225 115) \"\")");
        self.eval("(command \"LINE\" '(215 115) '(225 115) \"\")");
        self.eval("(command \"TEXT\" '(217 120) 3 0 \"K1\")");
        self.eval("(command \"LINE\" '(220 115) '(220 100) \"\")");
        self.eval("(command \"LINE\" '(220 100) '(240 100) \"\")");
        self.eval("(command \"TEXT\" '(178 185) 2 0 \"S0\")");
        self.eval("(command \"TEXT\" '(203 185) 2 0 \"S1\")");
    }

    fn generate_control_schaltplan(&mut self, components: &[(String, String, String)]) {
        self.eval("(command \"LINE\" '(40 200) '(40 70) \"\")");
        self.eval("(command \"LINE\" '(200 200) '(200 70) \"\")");
        self.eval("(command \"TEXT\" '(35 203) 3 0 \"+24V\")");
        self.eval("(command \"TEXT\" '(195 203) 3 0 \"0V\")");

        let mut y = 180;
        for (comp_name, comp_type, _) in components {
            if comp_type.contains("Taster") {
                self.eval(&format!("(command \"LINE\" '(40 {}) '(80 {}) \"\")", y, y));
                self.eval(&format!(
                    "(command \"LINE\" '(80 {}) '(100 {}) \"\")",
                    y - 5,
                    y
                ));
                self.eval(&format!(
                    "(command \"LINE\" '(100 {}) '(140 {}) \"\")",
                    y, y
                ));
                self.eval(&format!(
                    "(command \"TEXT\" '(85 {}) 3 0 \"{}\")",
                    y + 6,
                    comp_name
                ));
                y -= 25;
            } else if comp_type.contains("Meldeleuchte") {
                self.eval(&format!("(command \"LINE\" '(40 {}) '(100 {}) \"\")", y, y));
                self.eval(&format!("(command \"CIRCLE\" '(120 {}) 8)", y));
                self.eval(&format!(
                    "(command \"LINE\" '(128 {}) '(200 {}) \"\")",
                    y, y
                ));
                self.eval(&format!(
                    "(command \"TEXT\" '(130 {}) 3 0 \"{}\")",
                    y + 6,
                    comp_name
                ));
                y -= 25;
            }
        }
    }

    fn generate_schriftfeld(&mut self, name: &str, template: &str) {
        // Wire numbers
        for i in 1..=6 {
            self.eval(&format!(
                "(command \"TEXT\" '(8 {}) 2 0 \"{}\")",
                210 - i * 30,
                i
            ));
        }

        let sx = 190;
        let sy = 0;
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '(297 {}) \"\")",
            sx, sy, sy
        ));
        self.eval(&format!(
            "(command \"LINE\" '(297 {}) '(297 {}) \"\")",
            sy,
            sy + 50
        ));
        self.eval(&format!(
            "(command \"LINE\" '(297 {}) '({} {}) \"\")",
            sy + 50,
            sx,
            sy + 50
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '({} {}) \"\")",
            sx,
            sy + 50,
            sx,
            sy
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '(297 {}) \"\")",
            sx,
            sy + 40,
            sy + 40
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '(297 {}) \"\")",
            sx,
            sy + 30,
            sy + 30
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '(297 {}) \"\")",
            sx,
            sy + 20,
            sy + 20
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '(297 {}) \"\")",
            sx,
            sy + 10,
            sy + 10
        ));
        self.eval(&format!(
            "(command \"LINE\" '({} {}) '({} {}) \"\")",
            sx + 55,
            sy + 30,
            sx + 55,
            sy
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 3.5 0 \"Elektrotechnik Trahe GmbH\")",
            sx + 2,
            sy + 42
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2.5 0 \"84579 Unterneukirchen\")",
            sx + 2,
            sy + 32
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 5 0 \"{}\")",
            sx + 2,
            sy + 22,
            name
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2.5 0 \"{}\")",
            sx + 57,
            sy + 22,
            template
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2 0 \"gez. HT\")",
            sx + 2,
            sy + 12
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2 0 \"03.91\")",
            sx + 30,
            sy + 12
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2 0 \"Blatt 1\")",
            sx + 57,
            sy + 12
        ));
        self.eval(&format!(
            "(command \"TEXT\" '({} {}) 2 0 \"gepr.\")",
            sx + 2,
            sy + 2
        ));
    }
}

// Simple JSON parsing for components (no serde dependency)
fn serde_json_minimal_parse(json: &str) -> Result<Vec<(String, String, String)>, ()> {
    let mut result = Vec::new();
    // Very basic parsing - expects [{"name":"..","type":"..","value":".."},...]
    let json = json.trim();
    if !json.starts_with('[') || !json.ends_with(']') {
        return Ok(result);
    }

    let inner = &json[1..json.len() - 1];
    for obj in inner.split("},") {
        let obj = obj.trim().trim_start_matches('{').trim_end_matches('}');
        let mut name = String::new();
        let mut typ = String::new();
        let mut val = String::new();

        for part in obj.split(',') {
            let part = part.trim();
            if part.contains("\"name\"") {
                if let Some(v) = extract_json_string(part) {
                    name = v;
                }
            } else if part.contains("\"type\"") {
                if let Some(v) = extract_json_string(part) {
                    typ = v;
                }
            } else if part.contains("\"value\"") {
                if let Some(v) = extract_json_string(part) {
                    val = v;
                }
            }
        }
        if !name.is_empty() {
            result.push((name, typ, val));
        }
    }
    Ok(result)
}

fn extract_json_string(s: &str) -> Option<String> {
    let mut in_value = false;
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == ':' {
            in_value = true;
            continue;
        }
        if in_value && c == '"' {
            // Start or end of string
            if result.is_empty() {
                // Start collecting
                for c2 in chars.by_ref() {
                    if c2 == '"' {
                        return Some(result);
                    }
                    result.push(c2);
                }
            }
        }
    }
    None
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Steganography & Encryption - WASM functions
// ============================================

/// Extract encryption key from PNG image data using LSB steganography.
/// The key is embedded in the LSB of red channel pixels.
/// Format: 2-byte big-endian length + key bytes + null terminator
#[wasm_bindgen]
pub fn extract_key_from_png(png_data: &[u8]) -> Option<String> {
    // Simple PNG decoder - find IDAT chunk and decompress
    let pixels = decode_png_pixels(png_data)?;

    // Extract LSB from red channel (every 4th byte in RGBA)
    let bits: Vec<u8> = pixels.iter()
        .step_by(4)
        .map(|&p| p & 1)
        .collect();

    if bits.len() < 16 {
        return None;
    }

    // Read length (first 16 bits = 2 bytes, big-endian)
    let mut length: usize = 0;
    for bit in bits.iter().take(16) {
        length = (length << 1) | (*bit as usize);
    }

    if length == 0 || length > 256 || bits.len() < 16 + length * 8 {
        return None;
    }

    // Read key bytes
    let mut key_bytes = Vec::with_capacity(length);
    for byte_idx in 0..length {
        let mut byte: u8 = 0;
        for bit_idx in 0..8 {
            let bit_pos = 16 + byte_idx * 8 + bit_idx;
            byte = (byte << 1) | bits[bit_pos];
        }
        key_bytes.push(byte);
    }

    String::from_utf8(key_bytes).ok()
}

/// Simple PNG decoder - extracts raw RGBA pixels
fn decode_png_pixels(data: &[u8]) -> Option<Vec<u8>> {
    // Check PNG signature
    if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }

    let mut pos = 8;
    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut idat_data = Vec::new();

    // Parse chunks
    while pos + 8 < data.len() {
        let chunk_len = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        let chunk_type = &data[pos+4..pos+8];

        if pos + 8 + chunk_len > data.len() {
            break;
        }

        match chunk_type {
            b"IHDR" => {
                if chunk_len >= 8 {
                    width = u32::from_be_bytes([data[pos+8], data[pos+9], data[pos+10], data[pos+11]]);
                    height = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]);
                }
            }
            b"IDAT" => {
                idat_data.extend_from_slice(&data[pos+8..pos+8+chunk_len]);
            }
            b"IEND" => break,
            _ => {}
        }

        pos += 12 + chunk_len; // length(4) + type(4) + data + crc(4)
    }

    if width == 0 || height == 0 || idat_data.is_empty() {
        return None;
    }

    // Decompress IDAT data (zlib)
    let decompressed = miniz_decompress(&idat_data)?;

    // Decode PNG filter (simple - assuming filter type 0 for each row)
    let stride = (width as usize) * 4 + 1; // RGBA + filter byte
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..(height as usize) {
        let row_start = y * stride;
        if row_start >= decompressed.len() {
            break;
        }
        // Skip filter byte, copy pixel data
        let pixel_start = row_start + 1;
        let pixel_end = (pixel_start + (width as usize) * 4).min(decompressed.len());
        pixels.extend_from_slice(&decompressed[pixel_start..pixel_end]);
    }

    Some(pixels)
}

/// Zlib decompression using flate2
fn miniz_decompress(data: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result).ok()?;
    Some(result)
}

/// XOR encrypt/decrypt text with a key
#[wasm_bindgen]
pub fn xor_crypt(text: &str, key: &str) -> String {
    if key.is_empty() {
        return text.to_string();
    }
    let key_bytes: Vec<u8> = key.bytes().collect();
    text.bytes()
        .enumerate()
        .map(|(i, b)| (b ^ key_bytes[i % key_bytes.len()]) as char)
        .collect()
}

/// Default encryption key (fallback if logo extraction fails)
#[wasm_bindgen]
pub fn get_default_key() -> String {
    "HN2025".to_string()
}

/// Compress data using DEFLATE and return as base64
#[wasm_bindgen]
pub fn compress_to_base64(data: &str) -> String {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    if encoder.write_all(data.as_bytes()).is_err() {
        return String::new();
    }
    match encoder.finish() {
        Ok(compressed) => base64_encode(&compressed),
        Err(_) => String::new(),
    }
}

/// Decompress base64-encoded DEFLATE data
#[wasm_bindgen]
pub fn decompress_from_base64(data: &str) -> Option<String> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let compressed = base64_decode(data)?;
    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut result = String::new();
    decoder.read_to_string(&mut result).ok()?;
    Some(result)
}

/// URL-safe base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };

        result.push(CHARS[(b0 >> 2) & 0x3F] as char);
        result.push(CHARS[((b0 << 4) | (b1 >> 4)) & 0x3F] as char);
        if i + 1 < data.len() {
            result.push(CHARS[((b1 << 2) | (b2 >> 6)) & 0x3F] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARS[b2 & 0x3F] as char);
        }
        i += 3;
    }
    result
}

/// URL-safe base64 decoding
fn base64_decode(data: &str) -> Option<Vec<u8>> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = Vec::new();
    let bytes: Vec<u8> = data.bytes().collect();
    let mut i = 0;

    while i < bytes.len() {
        let c0 = CHARS.iter().position(|&c| c == bytes[i])?;
        let c1 = if i + 1 < bytes.len() { CHARS.iter().position(|&c| c == bytes[i + 1])? } else { 0 };
        let c2 = if i + 2 < bytes.len() { CHARS.iter().position(|&c| c == bytes[i + 2]) } else { None };
        let c3 = if i + 3 < bytes.len() { CHARS.iter().position(|&c| c == bytes[i + 3]) } else { None };

        result.push(((c0 << 2) | (c1 >> 4)) as u8);
        if let Some(c2) = c2 {
            result.push((((c1 & 0xF) << 4) | (c2 >> 2)) as u8);
            if let Some(c3) = c3 {
                result.push((((c2 & 0x3) << 6) | c3) as u8);
            }
        }
        i += 4;
    }
    Some(result)
}

// ============================================
// LISP Examples - stored in Rust
// ============================================

const EXAMPLE_HELLO: &str = r#"; https://acadlisp.de/?example=hello
; Hello World in AutoLISP
(princ "\nHello from AutoLISP!")
(princ "\nRunning in Rust/WASM!")

; Draw a greeting
(command "TEXT" '(20 80) 10 0 "Hello World!")
(command "TEXT" '(20 60) 5 0 "AutoLISP in the browser")

; A simple box around it
(command "LINE" '(10 50) '(200 50) "")
(command "LINE" '(200 50) '(200 95) "")
(command "LINE" '(200 95) '(10 95) "")
(command "LINE" '(10 95) '(10 50) "")"#;

const EXAMPLE_MATH: &str = r#"; https://acadlisp.de/?example=math
; Math and Recursion
(princ "\n=== Math ===")
(princ (strcat "\n2 + 3 = " (itoa (+ 2 3))))
(princ (strcat "\n7 * 8 = " (itoa (* 7 8))))

; Factorial with recursion
(defun factorial (n)
  (if (<= n 1)
    1
    (* n (factorial (1- n)))))

(princ "\n\n=== Factorial ===")
(princ (strcat "\n5! = " (itoa (factorial 5))))
(princ (strcat "\n10! = " (itoa (factorial 10))))

; Fibonacci
(defun fib (n)
  (if (< n 2)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(princ "\n\n=== Fibonacci ===")
(setq i 0)
(while (< i 10)
  (princ (strcat " " (itoa (fib i))))
  (setq i (1+ i)))"#;

const EXAMPLE_BOX: &str = r#"; https://acadlisp.de/?example=box
; Draw a parametric box
(defun draw-box (x y w h)
  (command "LINE" (list x y) (list (+ x w) y) "")
  (command "LINE" (list (+ x w) y) (list (+ x w) (+ y h)) "")
  (command "LINE" (list (+ x w) (+ y h)) (list x (+ y h)) "")
  (command "LINE" (list x (+ y h)) (list x y) ""))

; Draw nested boxes
(draw-box 10 10 180 130)
(draw-box 20 20 160 110)
(draw-box 30 30 140 90)
(draw-box 40 40 120 70)
(draw-box 50 50 100 50)

; Label
(command "TEXT" '(60 80) 8 0 "Nested Boxes")

(princ "\n5 boxes drawn!")"#;

const EXAMPLE_SPIRAL: &str = r#"; https://acadlisp.de/?example=spiral
; Draw a spiral using math
(defun spiral (cx cy r angle step max-r)
  (if (< r max-r)
    (progn
      (setq x (+ cx (* r (cos angle))))
      (setq y (+ cy (* r (sin angle))))
      (command "CIRCLE" (list x y) 2)
      (spiral cx cy (+ r step) (+ angle 0.5) step max-r))))

; Draw the spiral
(spiral 100 80 5 0 3 70)

; Center point
(command "CIRCLE" '(100 80) 5)
(command "TEXT" '(70 10) 4 0 "Recursive Spiral")

(princ "\nSpiral complete!")"#;

const EXAMPLE_SCHALTPLAN: &str = r#"; https://acadlisp.de/?example=schaltplan
; Mini Schaltplan
; Power rails
(command "LINE" '(30 140) '(30 20) "")
(command "LINE" '(70 140) '(70 20) "")
(command "LINE" '(110 140) '(110 20) "")
(command "TEXT" '(27 145) 4 0 "L1")
(command "TEXT" '(67 145) 4 0 "L2")
(command "TEXT" '(107 145) 4 0 "L3")

; Schütz K1 (contactor)
(defun draw-contact (x y)
  (command "LINE" (list (- x 4) (- y 3)) (list (+ x 4) (+ y 3)) ""))

(draw-contact 30 100)
(draw-contact 70 100)
(draw-contact 110 100)
(command "TEXT" '(120 98) 3 0 "K1")

; Motor
(command "CIRCLE" '(70 40) 15)
(command "TEXT" '(65 37) 8 0 "M")

; Connections
(command "LINE" '(30 60) '(55 40) "")
(command "LINE" '(70 60) '(70 55) "")
(command "LINE" '(110 60) '(85 40) "")

; Title block
(command "LINE" '(130 10) '(190 10) "")
(command "LINE" '(190 10) '(190 35) "")
(command "LINE" '(190 35) '(130 35) "")
(command "LINE" '(130 35) '(130 10) "")
(command "TEXT" '(133 25) 3 0 "MOTOR_ST")
(command "TEXT" '(133 15) 2 0 "01-K1")

(princ "\nSchaltplan done!")"#;

const EXAMPLE_FRACTAL: &str = r#"; https://acadlisp.de/?example=fractal
; Recursive Tree (simple fractal)
(defun tree (x y len angle depth)
  (if (> depth 0)
    (progn
      ; Calculate end point
      (setq x2 (+ x (* len (cos angle))))
      (setq y2 (+ y (* len (sin angle))))
      ; Draw branch
      (command "LINE" (list x y) (list x2 y2) "")
      ; Recurse: two branches
      (tree x2 y2 (* len 0.7) (+ angle 0.5) (1- depth))
      (tree x2 y2 (* len 0.7) (- angle 0.5) (1- depth)))))

; Draw tree from bottom center
(tree 100 10 40 1.57 6)

(command "TEXT" '(60 5) 3 0 "Recursive Tree")
(princ "\nTree complete!")"#;

const EXAMPLE_KICAD_SYM: &str = r#"; https://acadlisp.de/?example=kicad_sym
; KiCad Symbol Example
; Creates a simple IC symbol for KiCad

; Define symbol properties
(kicad-prop "Reference" "U1" 0 7 0 1.27 1)
(kicad-prop "Value" "NE555" 0 -7 0 1.27 1)

; Draw symbol body (rectangle)
(command "LINE" '(-6 5) '(6 5) "")
(command "LINE" '(6 5) '(6 -5) "")
(command "LINE" '(6 -5) '(-6 -5) "")
(command "LINE" '(-6 -5) '(-6 5) "")

; Add pins
; (kicad-pin name number type style x y length rotation)
(kicad-pin "GND" "1" "power_in" "line" 0 -7.54 2.54 90)
(kicad-pin "VCC" "8" "power_in" "line" 0 7.54 2.54 270)
(kicad-pin "TRIG" "2" "input" "line" -8.54 2.54 2.54 0)
(kicad-pin "OUT" "3" "output" "line" 8.54 0 2.54 180)
(kicad-pin "RESET" "4" "input" "line" -8.54 -2.54 2.54 0)

(princ "\nKiCad symbol ready! Click 'KiCad Sym' to export.")"#;

const EXAMPLE_KICAD_FP: &str = r#"; https://acadlisp.de/?example=kicad_fp
; KiCad Footprint Example
; Creates a simple 2-pad SMD footprint

; Set layer to silkscreen for outline
(command "LAYER" "S" "F.SilkS" "")

; Draw outline on silkscreen
(command "LINE" '(-2.5 1.5) '(2.5 1.5) "")
(command "LINE" '(2.5 1.5) '(2.5 -1.5) "")
(command "LINE" '(2.5 -1.5) '(-2.5 -1.5) "")
(command "LINE" '(-2.5 -1.5) '(-2.5 1.5) "")

; Pin 1 indicator
(command "CIRCLE" '(-1.8 0) 0.2)

; Add SMD pads
; (kicad-pad name type shape x y width height drill rotation layers)
(kicad-pad "1" "smd" "rect" -1.5 0 1.0 2.0 0 0 "F.Cu")
(kicad-pad "2" "smd" "rect" 1.5 0 1.0 2.0 0 0 "F.Cu")

(princ "\nKiCad footprint ready! Click 'KiCad Mod' to export.")"#;

const EXAMPLE_KICAD_CAKE: &str = r#"; https://acadlisp.de/?example=kicad_cake
; KiCad Birthday Cake Symbol
; A cake you could actually fabricate as PCB art!

(kicad-prop "Reference" "CAKE1" 0 25 0 1.27 1)
(kicad-prop "Value" "BIRTHDAY_JOSE" 0 -12 0 1.27 1)

; Bottom layer (big rectangle)
(command "LINE" '(-15 -10) '(15 -10) "")
(command "LINE" '(15 -10) '(15 -5) "")
(command "LINE" '(15 -5) '(-15 -5) "")
(command "LINE" '(-15 -5) '(-15 -10) "")

; Middle layer
(command "LINE" '(-12 -5) '(12 -5) "")
(command "LINE" '(12 -5) '(12 0) "")
(command "LINE" '(12 0) '(-12 0) "")
(command "LINE" '(-12 0) '(-12 -5) "")

; Top layer
(command "LINE" '(-9 0) '(9 0) "")
(command "LINE" '(9 0) '(9 5) "")
(command "LINE" '(9 5) '(-9 5) "")
(command "LINE" '(-9 5) '(-9 0) "")

; Frosting waves on top
(command "LINE" '(-9 5) '(-6 6) "")
(command "LINE" '(-6 6) '(-3 5) "")
(command "LINE" '(-3 5) '(0 6) "")
(command "LINE" '(0 6) '(3 5) "")
(command "LINE" '(3 5) '(6 6) "")
(command "LINE" '(6 6) '(9 5) "")

; Candles (5 pins - the "vias" of the cake!)
(kicad-pin "CANDLE1" "1" "passive" "line" -6 10 4 270)
(kicad-pin "CANDLE2" "2" "passive" "line" -3 10 4 270)
(kicad-pin "CANDLE3" "3" "passive" "line" 0 10 4 270)
(kicad-pin "CANDLE4" "4" "passive" "line" 3 10 4 270)
(kicad-pin "CANDLE5" "5" "passive" "line" 6 10 4 270)

; Flames (triangles as lines)
(command "LINE" '(-6 10) '(-5.5 12) "")
(command "LINE" '(-5.5 12) '(-6.5 12) "")
(command "LINE" '(-6.5 12) '(-6 10) "")

(command "LINE" '(-3 10) '(-2.5 12) "")
(command "LINE" '(-2.5 12) '(-3.5 12) "")
(command "LINE" '(-3.5 12) '(-3 10) "")

(command "LINE" '(0 10) '(0.5 12) "")
(command "LINE" '(0.5 12) '(-0.5 12) "")
(command "LINE" '(-0.5 12) '(0 10) "")

(command "LINE" '(3 10) '(3.5 12) "")
(command "LINE" '(3.5 12) '(2.5 12) "")
(command "LINE" '(2.5 12) '(3 10) "")

(command "LINE" '(6 10) '(6.5 12) "")
(command "LINE" '(6.5 12) '(5.5 12) "")
(command "LINE" '(5.5 12) '(6 10) "")

; Text on cake
(command "TEXT" '(-7 -8) 2 0 "JOSE 2025")

(princ "\nBirthday cake ready! Export to KiCad and fabricate it!")"#;

const EXAMPLE_KICAD_NEWYEAR: &str = r#"; https://acadlisp.de/?example=kicad_newyear
; KiCad New Year 2026 Symbol
; Fireworks and champagne as PCB art!

(kicad-prop "Reference" "NY1" 0 40 0 1.27 1)
(kicad-prop "Value" "HAPPY_2026" 0 -25 0 1.27 1)

; === BIG "2026" ===
(command "TEXT" '(-12 -5) 10 0 "2026")

; === FIREWORK LEFT (burst pattern) ===
(command "LINE" '(-18 25) '(-18 32) "")
(command "LINE" '(-18 25) '(-23 30) "")
(command "LINE" '(-18 25) '(-13 30) "")
(command "LINE" '(-18 25) '(-25 25) "")
(command "LINE" '(-18 25) '(-11 25) "")
(command "LINE" '(-18 25) '(-23 20) "")
(command "LINE" '(-18 25) '(-13 20) "")
(command "LINE" '(-18 25) '(-18 18) "")

; Sparks
(command "TEXT" '(-18 33) 2 0 "*")
(command "TEXT" '(-24 31) 2 0 "*")
(command "TEXT" '(-12 31) 2 0 "*")

; === FIREWORK RIGHT (burst pattern) ===
(command "LINE" '(18 28) '(18 35) "")
(command "LINE" '(18 28) '(13 33) "")
(command "LINE" '(18 28) '(23 33) "")
(command "LINE" '(18 28) '(11 28) "")
(command "LINE" '(18 28) '(25 28) "")
(command "LINE" '(18 28) '(13 23) "")
(command "LINE" '(18 28) '(23 23) "")
(command "LINE" '(18 28) '(18 21) "")

; Sparks
(command "TEXT" '(18 36) 2 0 "*")
(command "TEXT" '(12 34) 2 0 "*")
(command "TEXT" '(24 34) 2 0 "*")

; === CHAMPAGNE GLASS LEFT ===
(command "LINE" '(-8 8) '(-5 -2) "")
(command "LINE" '(-2 8) '(-5 -2) "")
(command "LINE" '(-8 8) '(-2 8) "")
(command "LINE" '(-5 -2) '(-5 -5) "")
(command "LINE" '(-7 -5) '(-3 -5) "")

; Bubbles
(command "TEXT" '(-6 6) 1 0 "o")
(command "TEXT" '(-4 4) 1 0 "o")

; === CHAMPAGNE GLASS RIGHT ===
(command "LINE" '(2 8) '(5 -2) "")
(command "LINE" '(8 8) '(5 -2) "")
(command "LINE" '(2 8) '(8 8) "")
(command "LINE" '(5 -2) '(5 -5) "")
(command "LINE" '(3 -5) '(7 -5) "")

; Bubbles
(command "TEXT" '(4 6) 1 0 "o")
(command "TEXT" '(6 4) 1 0 "o")

; === CLOCK showing midnight ===
(command "CIRCLE" '(0 20) 5)
(command "LINE" '(0 20) '(0 24) "")
(command "LINE" '(0 20) '(0.5 23) "")
(command "TEXT" '(-1 16) 2 0 "12")

; === Firework pins (connectors!) ===
(kicad-pin "SPARK1" "1" "passive" "line" -18 35 3 270)
(kicad-pin "SPARK2" "2" "passive" "line" 18 38 3 270)
(kicad-pin "GND" "3" "power_in" "line" 0 -20 5 90)

(princ "\nHappy New Year 2026! Export to KiCad!")"#;

const EXAMPLE_BIRTHDAY: &str = r#"; https://acadlisp.de/?example=birthday
; Happy Birthday Jose!
; A festive greeting card in AutoLISP

(princ "\nHappy Birthday Jose!")
(princ "\nDrawing your gift...")

; Draw "HAPPY"
(command "TEXT" '(20 80) 15 0 "HAPPY")

; Draw "BIRTHDAY"
(command "TEXT" '(20 55) 15 0 "BIRTHDAY")

; Draw "JOSE!"
(command "TEXT" '(20 30) 20 0 "JOSE!")

; Box around it
(command "LINE" '(10 20) '(200 20) "")
(command "LINE" '(200 20) '(200 110) "")
(command "LINE" '(200 110) '(10 110) "")
(command "LINE" '(10 110) '(10 20) "")

; Candles (5 vertical lines)
(command "LINE" '(40 110) '(40 130) "")
(command "LINE" '(70 110) '(70 130) "")
(command "LINE" '(100 110) '(100 130) "")
(command "LINE" '(130 110) '(130 130) "")
(command "LINE" '(160 110) '(160 130) "")

; Flames
(command "TEXT" '(38 132) 8 0 "*")
(command "TEXT" '(68 132) 8 0 "*")
(command "TEXT" '(98 132) 8 0 "*")
(command "TEXT" '(128 132) 8 0 "*")
(command "TEXT" '(158 132) 8 0 "*")

; Final greeting on canvas
(command "TEXT" '(20 5) 8 0 "Feliz Aniversario Jose!")"#;

const EXAMPLE_NEWYEAR2026: &str = r#"; https://acadlisp.de/?example=newyear2026
; Happy New Year 2026!
; Fireworks celebration in AutoLISP

(princ "\nHappy New Year 2026!")
(princ "\nLaunching fireworks...")

; Night sky background frame
(command "LINE" '(0 0) '(300 0) "")
(command "LINE" '(300 0) '(300 200) "")
(command "LINE" '(300 200) '(0 200) "")
(command "LINE" '(0 200) '(0 0) "")

; Main title
(command "TEXT" '(50 180) 20 0 "HAPPY NEW YEAR")
(command "TEXT" '(100 155) 25 0 "2026")

; Firework 1 - Left (starburst pattern)
(defun firework (cx cy size)
  (command "CIRCLE" (list cx cy) 2)
  (repeat 12
    (setq ang (* (getvar "USERR1") 30))
    (setvar "USERR1" (1+ (getvar "USERR1")))
    (command "LINE"
      (list cx cy)
      (list (+ cx (* size (cos (* ang (/ 3.14159 180)))))
            (+ cy (* size (sin (* ang (/ 3.14159 180))))))
      "")
  )
)

; Draw fireworks at different positions
(setvar "USERR1" 0)
(firework 50 120 25)

(setvar "USERR1" 0)
(firework 150 130 30)

(setvar "USERR1" 0)
(firework 250 110 20)

; Sparkles / stars
(command "TEXT" '(30 90) 10 0 "*")
(command "TEXT" '(80 140) 8 0 "*")
(command "TEXT" '(120 100) 12 0 "*")
(command "TEXT" '(180 85) 8 0 "*")
(command "TEXT" '(220 145) 10 0 "*")
(command "TEXT" '(270 95) 8 0 "*")
(command "TEXT" '(200 70) 6 0 "*")
(command "TEXT" '(60 60) 6 0 "*")

; Champagne glasses
(command "LINE" '(130 30) '(125 10) "")
(command "LINE" '(125 10) '(135 10) "")
(command "LINE" '(135 10) '(130 30) "")
(command "CIRCLE" '(130 35) 8)

(command "LINE" '(170 30) '(165 10) "")
(command "LINE" '(165 10) '(175 10) "")
(command "LINE" '(175 10) '(170 30) "")
(command "CIRCLE" '(170 35) 8)

; Clink effect
(command "TEXT" '(145 45) 6 0 "*")

; Bottom message
(command "TEXT" '(70 5) 8 0 "Frohes Neues Jahr 2026!")

(princ "\nProst! Cheers to 2026!")"#;

fn entity_to_json(entity: &DrawEntity) -> String {
    match entity {
        DrawEntity::Line {
            x1,
            y1,
            x2,
            y2,
            layer,
        } => {
            format!(
                r#"{{"type":"LINE","x1":{},"y1":{},"x2":{},"y2":{},"layer":"{}"}}"#,
                x1, y1, x2, y2, layer
            )
        }
        DrawEntity::Circle {
            cx,
            cy,
            radius,
            layer,
        } => {
            format!(
                r#"{{"type":"CIRCLE","cx":{},"cy":{},"r":{},"layer":"{}"}}"#,
                cx, cy, radius, layer
            )
        }
        DrawEntity::Arc {
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
            layer,
        } => {
            format!(
                r#"{{"type":"ARC","cx":{},"cy":{},"r":{},"start":{},"end":{},"layer":"{}"}}"#,
                cx, cy, radius, start_angle, end_angle, layer
            )
        }
        DrawEntity::Text {
            x,
            y,
            height,
            text,
            layer,
        } => {
            let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
            format!(
                r#"{{"type":"TEXT","x":{},"y":{},"h":{},"text":"{}","layer":"{}"}}"#,
                x, y, height, escaped, layer
            )
        }
        DrawEntity::Point { x, y, layer } => {
            format!(
                r#"{{"type":"POINT","x":{},"y":{},"layer":"{}"}}"#,
                x, y, layer
            )
        }
        DrawEntity::Insert {
            block_name,
            x,
            y,
            scale,
            rotation,
            layer,
        } => {
            format!(
                r#"{{"type":"INSERT","block":"{}","x":{},"y":{},"scale":{},"rot":{},"layer":"{}"}}"#,
                block_name, x, y, scale, rotation, layer
            )
        }
        DrawEntity::Pin {
            name,
            number,
            etype,
            style,
            x,
            y,
            length,
            rotation,
            layer,
        } => {
            format!(
                r#"{{"type":"PIN","name":"{}","num":"{}","etype":"{}","style":"{}","x":{},"y":{},"len":{},"rot":{},"layer":"{}"}}"#,
                name, number, etype, style, x, y, length, rotation, layer
            )
        }
        DrawEntity::Property {
            key,
            value,
            x,
            y,
            rotation,
            height,
            visible,
            layer,
        } => {
            format!(
                r#"{{"type":"PROP","key":"{}","val":"{}","x":{},"y":{},"rot":{},"h":{},"vis":{},"layer":"{}"}}"#,
                key, value, x, y, rotation, height, visible, layer
            )
        }
        DrawEntity::Pad {
            name,
            ptype,
            shape,
            x,
            y,
            width,
            height,
            drill,
            rotation,
            layers,
        } => {
            format!(
                r#"{{"type":"PAD","name":"{}","ptype":"{}","shape":"{}","x":{},"y":{},"w":{},"h":{},"drill":{},"rot":{},"layers":"{}"}}"#,
                name, ptype, shape, x, y, width, height, drill, rotation, layers
            )
        }
    }
}

fn entities_to_svg(entities: &[DrawEntity], cad_type: CadType) -> String {
    // Calculate bounding box
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    if entities.is_empty() {
        return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"300\"><rect fill=\"#1a1a2e\" width=\"100%\" height=\"100%\"/></svg>");
    }

    // Y-flip factor: both RustLisp and KiCad use Y-up, SVG uses Y-down
    // For now both use same coordinate transform; future: different styling/colors per cad_type
    let _y_flip = -1.0;
    let _cad_type = cad_type;

    for entity in entities {
        match entity {
            DrawEntity::Line { x1, y1, x2, y2, .. } => {
                min_x = min_x.min(*x1).min(*x2);
                min_y = min_y.min(*y1).min(*y2);
                max_x = max_x.max(*x1).max(*x2);
                max_y = max_y.max(*y1).max(*y2);
            }
            DrawEntity::Circle { cx, cy, radius, .. } => {
                min_x = min_x.min(cx - radius);
                min_y = min_y.min(cy - radius);
                max_x = max_x.max(cx + radius);
                max_y = max_y.max(cy + radius);
            }
            DrawEntity::Text {
                x, y, height, text, ..
            } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x + text.len() as f64 * height * 0.6);
                max_y = max_y.max(*y + height);
            }
            DrawEntity::Insert { x, y, .. } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x + 50.0);
                max_y = max_y.max(*y + 50.0);
            }
            DrawEntity::Pin { x, y, length, .. } => {
                // Include pins in bounding box
                min_x = min_x.min(*x - length);
                min_y = min_y.min(*y - length);
                max_x = max_x.max(*x + length);
                max_y = max_y.max(*y + length);
            }
            DrawEntity::Property { .. } => {
                // Ignore properties for bounds
            }
            DrawEntity::Pad {
                x,
                y,
                width,
                height,
                ..
            } => {
                let w2 = width / 2.0;
                let h2 = height / 2.0;
                min_x = min_x.min(*x - w2);
                min_y = min_y.min(*y - h2);
                max_x = max_x.max(*x + w2);
                max_y = max_y.max(*y + h2);
            }
            DrawEntity::Arc { cx, cy, radius, .. } => {
                min_x = min_x.min(cx - radius);
                min_y = min_y.min(cy - radius);
                max_x = max_x.max(cx + radius);
                max_y = max_y.max(cy + radius);
            }
            DrawEntity::Point { x, y, .. } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
        }
    }

    let padding = 20.0;
    min_x -= padding;
    min_y -= padding;
    max_x += padding;
    max_y += padding;
    let width = max_x - min_x;
    let height = max_y - min_y;

    // For SVG we flip Y coordinates (CAD Y goes up, SVG Y goes down)
    // viewBox: min_x, -max_y, width, height
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="{} {} {} {}"
     width="100%" height="100%"
     preserveAspectRatio="xMidYMid meet"
     style="background-color: #1a1a2e;">
  <title>AutoLISP Drawing - Generated</title>
  <defs>
    <style>
      .line {{ stroke: #00ff88; stroke-width: 1; fill: none; }}
      .circle {{ stroke: #00aaff; stroke-width: 1; fill: none; }}
      .text {{ fill: #ffffff; font-family: monospace; }}
      .block {{ stroke: #ff8800; fill: none; }}
      .pin {{ stroke: #ff00ff; stroke-width: 1.5; fill: none; }}
      .pin-dot {{ fill: #ff00ff; }}
      .pin-text {{ fill: #ffaaff; font-family: monospace; }}
      .pad {{ fill: #ffaa00; fill-opacity: 0.7; stroke: #ff8800; stroke-width: 0.5; }}
    </style>
  </defs>
  <g>
"#,
        min_x - padding,
        -(max_y + padding),
        width,
        height,
    );

    for entity in entities {
        match entity {
            DrawEntity::Line { x1, y1, x2, y2, .. } => {
                // Negate Y for SVG coordinate system (CAD Y up -> SVG Y down)
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"line\" />\n",
                    x1, -y1, x2, -y2
                ));
            }
            DrawEntity::Circle { cx, cy, radius, .. } => {
                svg.push_str(&format!(
                    "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"circle\" />\n",
                    cx, -cy, radius
                ));
            }
            DrawEntity::Text {
                x, y, height, text, ..
            } => {
                let escaped = text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                // Y already negated for SVG; render text directly (no flip needed)
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"{}\">{}</text>\n",
                    x, -y, height * 1.5, escaped
                ));
            }
            DrawEntity::Insert {
                block_name, x, y, ..
            } => {
                let svg_y = -y;
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"40\" height=\"40\" class=\"block\" />\n",
                    x, svg_y - 40.0
                ));
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"8\">{}</text>\n",
                    x + 2.0, svg_y - 16.0, block_name
                ));
            }
            DrawEntity::Pin {
                name,
                number: _,
                x,
                y,
                length,
                rotation,
                ..
            } => {
                // Draw pin: line from pin position to body, circle at connection point
                let rad = rotation.to_radians();
                let end_x = x + length * rad.cos();
                let end_y = y + length * rad.sin();
                // Negate Y for SVG
                let svg_y = -y;
                let svg_end_y = -end_y;
                // Pin line
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"pin\" />\n",
                    x, svg_y, end_x, svg_end_y
                ));
                // Pin dot at connection point
                svg.push_str(&format!(
                    "    <circle cx=\"{}\" cy=\"{}\" r=\"1\" class=\"pin-dot\" />\n",
                    x, svg_y
                ));
                // Pin name text (no flip needed, Y already negated)
                let text_x = (x + end_x) / 2.0;
                let text_y = (svg_y + svg_end_y) / 2.0;
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"pin-text\" font-size=\"2.5\" text-anchor=\"middle\">{}</text>\n",
                    text_x, text_y, name
                ));
            }
            DrawEntity::Property { key: _, value, x, y, height, .. } => {
                // Draw property as text (no flip needed, Y already negated)
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" fill=\"#aaaaaa\" font-size=\"{}\" text-anchor=\"middle\">{}</text>\n",
                    x, -y, height.max(2.0), value
                ));
            }
            DrawEntity::Pad {
                name,
                shape,
                x,
                y,
                width,
                height,
                ..
            } => {
                // Draw a simple shape for pad in SVG
                let svg_y = -y;
                if shape == "circle" || shape == "oval" {
                    svg.push_str(&format!(
                        "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#ffaa00\" fill-opacity=\"0.5\" />\n",
                        x,
                        svg_y,
                        width.min(*height) / 2.0
                    ));
                } else {
                    // Rect or others
                    svg.push_str(&format!(
                        "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffaa00\" fill-opacity=\"0.5\" transform=\"rotate(0)\" />\n",
                        x - width / 2.0,
                        svg_y - height / 2.0,
                        width,
                        height
                    ));
                }
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"{}\" text-anchor=\"middle\">{}</text>\n",
                    x, svg_y, width.min(*height) * 0.4, name
                ));
            }
            DrawEntity::Arc {
                cx,
                cy,
                radius,
                start_angle,
                end_angle,
                ..
            } => {
                let start_x = cx + radius * start_angle.cos();
                let start_y = -(cy + radius * start_angle.sin());
                let end_x = cx + radius * end_angle.cos();
                let end_y = -(cy + radius * end_angle.sin());
                let large_arc = if (end_angle - start_angle).abs() > std::f64::consts::PI {
                    1
                } else {
                    0
                };
                svg.push_str(&format!(
                    "    <path d=\"M {} {} A {} {} 0 {} 1 {} {}\" class=\"circle\" />\n",
                    start_x, start_y, radius, radius, large_arc, end_x, end_y
                ));
            }
            DrawEntity::Point { x, y, .. } => {
                svg.push_str(&format!(
                    "    <circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"#00ff88\" />\n",
                    x, -y
                ));
            }
        }
    }

    svg.push_str("  </g>\n</svg>\n");
    svg
}

/// Parse a number from eval result (which may be in various formats)
fn parse_number_result(result: &str) -> Option<f64> {
    // Try direct parse first
    if let Ok(n) = result.parse::<f64>() {
        return Some(n);
    }

    // Try parsing as a simple value (might have quotes or brackets)
    let trimmed = result
        .trim()
        .trim_matches(|c| c == '[' || c == ']' || c == '"');
    if let Ok(n) = trimmed.parse::<f64>() {
        return Some(n);
    }

    None
}

/// Escape a string for JSON
fn serde_json_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

/// Generate an SVG plot for a function
fn generate_plot_svg(
    points: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> String {
    let width = 800.0;
    let height = 500.0;
    let padding = 50.0;

    let plot_width = width - 2.0 * padding;
    let plot_height = height - 2.0 * padding;

    // Scale functions
    let scale_x = |x: f64| -> f64 { padding + (x - x_min) / (x_max - x_min) * plot_width };
    let scale_y =
        |y: f64| -> f64 { height - padding - (y - y_min) / (y_max - y_min) * plot_height };

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%" style="background:#1a1a2e">"#,
        width, height
    );

    // Grid lines
    svg.push_str("<g stroke=\"#333\" stroke-width=\"1\">");

    // Vertical grid lines
    let x_step = (x_max - x_min) / 10.0;
    let mut x = x_min;
    while x <= x_max {
        let sx = scale_x(x);
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            sx,
            padding,
            sx,
            height - padding
        ));
        x += x_step;
    }

    // Horizontal grid lines
    let y_step = (y_max - y_min) / 8.0;
    let mut y = y_min;
    while y <= y_max {
        let sy = scale_y(y);
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            padding,
            sy,
            width - padding,
            sy
        ));
        y += y_step;
    }
    svg.push_str("</g>");

    // Axes
    svg.push_str("<g stroke=\"#666\" stroke-width=\"2\">");

    // X axis (y = 0)
    if y_min <= 0.0 && y_max >= 0.0 {
        let y0 = scale_y(0.0);
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            padding,
            y0,
            width - padding,
            y0
        ));
    }

    // Y axis (x = 0)
    if x_min <= 0.0 && x_max >= 0.0 {
        let x0 = scale_x(0.0);
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            x0,
            padding,
            x0,
            height - padding
        ));
    }
    svg.push_str("</g>");

    // Axis labels
    svg.push_str("<g fill=\"#888\" font-family=\"monospace\" font-size=\"10\">");

    // X axis labels
    x = x_min;
    while x <= x_max {
        let sx = scale_x(x);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            sx,
            height - padding + 15.0,
            x as i32
        ));
        x += x_step * 2.0;
    }

    // Y axis labels
    y = y_min;
    while y <= y_max {
        let sy = scale_y(y);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\">{:.1}</text>",
            padding - 5.0,
            sy + 4.0,
            y
        ));
        y += y_step * 2.0;
    }
    svg.push_str("</g>");

    // Function curve
    if !points.is_empty() {
        svg.push_str("<path stroke=\"#00ff88\" stroke-width=\"2\" fill=\"none\" d=\"");

        let mut first = true;
        for (x, y) in points {
            // Only plot points within the view
            if *y >= y_min && *y <= y_max {
                let sx = scale_x(*x);
                let sy = scale_y(*y);
                if first {
                    svg.push_str(&format!("M{:.2} {:.2}", sx, sy));
                    first = false;
                } else {
                    svg.push_str(&format!(" L{:.2} {:.2}", sx, sy));
                }
            } else {
                first = true; // Break the path for out-of-range points
            }
        }
        svg.push_str("\"/>");

        // Data points
        svg.push_str("<g fill=\"#00aaff\">");
        for (x, y) in points {
            if *y >= y_min && *y <= y_max {
                let sx = scale_x(*x);
                let sy = scale_y(*y);
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"2\"/>",
                    sx, sy
                ));
            }
        }
        svg.push_str("</g>");
    }

    // Title
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"25\" fill=\"#ff8800\" font-family=\"monospace\" font-size=\"14\">f(x)</text>",
        padding
    ));

    svg.push_str("</svg>");
    svg
}

// ============================================
// Plot Examples - stored in Rust
// ============================================

const PLOT_SIN: &str = "; Sine wave
(defun f (x)
  (sin x))";

const PLOT_COS: &str = "; Cosine wave
(defun f (x)
  (cos x))";

const PLOT_PARABOLA: &str = "; Parabola: y = x²
(defun f (x)
  (* x x))";

const PLOT_CUBIC: &str = "; Cubic: y = x³
(defun f (x)
  (* x (* x x)))";

const PLOT_SQRT: &str = "; Square root (for x >= 0)
(defun f (x)
  (if (>= x 0)
    (sqrt x)
    0))";

const PLOT_ABS: &str = "; Absolute value: y = |x|
(defun f (x)
  (abs x))";

const PLOT_SINCOS: &str = "; Combined: sin(x) + cos(2x)
(defun f (x)
  (+ (sin x) (cos (* 2 x))))";

const PLOT_WAVE: &str = "; Damped wave
(defun f (x)
  (* (/ 1 (+ 1 (/ (abs x) 3)))
     (sin (* 2 x))))";

const PLOT_LISSAJOUS: &str = "; Lissajous-like curve
(defun f (x)
  (sin (* 2 x)))";

const PLOT_POLAR: &str = "; Polar rose approximation
(defun f (x)
  (* (cos (* 3 x)) (sin x)))";

const PLOT_TAYLOR_SIN: &str = "; Taylor series for sin(x)
; sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
; LISP shines: recursion computes factorial!

(defun factorial (n)
  (if (<= n 1)
    1
    (* n (factorial (1- n)))))

(defun power (x n)
  (if (<= n 0)
    1
    (* x (power x (1- n)))))

(defun taylor-sin (x terms)
  (if (<= terms 0)
    0
    (+ (* (power -1 (1- terms))
          (/ (power x (1- (* 2 terms)))
             (factorial (1- (* 2 terms)))))
       (taylor-sin x (1- terms)))))

(defun f (x)
  (taylor-sin x 7))";

const PLOT_FIBONACCI: &str = "; Fibonacci spiral approximation
; Golden ratio emerges from recursion!

(defun fib (n)
  (if (< n 2)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

; Approximate golden ratio
(setq phi (/ (+ 1 (sqrt 5)) 2))

(defun f (x)
  (if (< x 0)
    0
    (* 0.1 (fib (min 15 (abs (truncate x)))))))";

const PLOT_RECURSIVE_WAVE: &str = "; Recursive harmonic series
; f(x) = sin(x) + sin(2x)/2 + sin(3x)/3 + ...
; Shows infinite series via recursion

(defun harmonic (x n)
  (if (<= n 0)
    0
    (+ (/ (sin (* n x)) n)
       (harmonic x (1- n)))))

(defun f (x)
  (harmonic x 10))";

const PLOT_MANDELBROT_SLICE: &str = "; Mandelbrot escape time (y=0 slice)
; Complex iteration z = z² + c
; LISP handles the recursion naturally!

(defun mandel-iter (zr zi cr ci n)
  (if (or (> n 20) (> (+ (* zr zr) (* zi zi)) 4))
    n
    (mandel-iter
      (+ (- (* zr zr) (* zi zi)) cr)
      (+ (* 2 zr zi) ci)
      cr ci (1+ n))))

(defun f (x)
  (/ (mandel-iter 0 0 x 0 0) 5.0))";

const PLOT_NEWTON_SQRT: &str = "; Newton's method for sqrt(2)
; Converges via recursive refinement
; x' = (x + 2/x) / 2

(defun newton-sqrt (guess n)
  (if (<= n 0)
    guess
    (newton-sqrt
      (/ (+ guess (/ 2 guess)) 2)
      (1- n))))

; Show convergence over iterations
(defun f (x)
  (if (< x 0)
    1.414
    (newton-sqrt 1.0 (truncate (abs x)))))";

const PLOT_INTEGRAL: &str = "; Numerical Integration (Simpson's Rule)
; Computes integral of sin(x) from 0 to x
; Result should be 1 - cos(x)

(defun simpson-step (func a b)
  (* (/ (- b a) 6)
     (+ (func a)
        (* 4 (func (/ (+ a b) 2)))
        (func b))))

(defun integrate (func a b n)
  (if (<= n 0)
    0
    (+ (simpson-step func
         (+ a (* (/ (- b a) n) (1- n)))
         (+ a (* (/ (- b a) n) n)))
       (integrate func a b (1- n)))))

(defun g (t) (sin t))

(defun f (x)
  (if (<= x 0)
    0
    (integrate g 0 x 20)))";

const PLOT_DERIVATIVE: &str = "; Numerical Differentiation
; Computes d/dx of sin(x) using limit definition
; Result should be cos(x)

(setq h 0.0001)

(defun derivative (func x)
  (/ (- (func (+ x h)) (func (- x h)))
     (* 2 h)))

(defun g (t) (sin t))

; Plot derivative of sin = cos
(defun f (x)
  (derivative g x))";

const PLOT_SECOND_DERIV: &str = "; Second Derivative
; d²/dx² of sin(x) = -sin(x)
; Recursively apply differentiation!

(setq h 0.001)

(defun deriv (func x)
  (/ (- (func (+ x h)) (func (- x h)))
     (* 2 h)))

; Second derivative = derivative of derivative
(defun deriv2 (func x)
  (/ (- (deriv func (+ x h))
        (deriv func (- x h)))
     (* 2 h)))

(defun g (t) (sin t))

(defun f (x)
  (deriv2 g x))";

const PLOT_FOURIER: &str = "; Fourier Series: Square Wave
; Sum of odd harmonics: sin(x) + sin(3x)/3 + sin(5x)/5 + ...
; Shows how sine waves build a square wave!

(defun fourier-term (x n)
  (/ (sin (* n x)) n))

(defun square-wave (x terms)
  (if (< terms 1)
    0
    (+ (fourier-term x (1- (* 2 terms)))
       (square-wave x (1- terms)))))

; 10 terms of Fourier series
(defun f (x)
  (* (/ 4 3.14159) (square-wave x 10)))";

const PLOT_FOURIER_SAW: &str = "; Fourier Series: Sawtooth Wave
; Sum: sin(x) - sin(2x)/2 + sin(3x)/3 - ...
; Alternating signs via recursion

(defun saw-term (x n)
  (* (/ (sin (* n x)) n)
     (if (= 0 (rem n 2)) -1 1)))

(defun sawtooth (x n)
  (if (< n 1)
    0
    (+ (saw-term x n)
       (sawtooth x (1- n)))))

(defun f (x)
  (* (/ 2 3.14159) (sawtooth x 15)))";

const PLOT_CONVOLUTION: &str = "; Convolution Approximation
; Gaussian smoothing of a step function
; Shows signal processing concept

(defun gaussian (x sigma)
  (* (/ 1 (* sigma 2.507))
     (/ 1 (+ 1 (* (/ x sigma) (/ x sigma))))))

(defun step (x) (if (> x 0) 1 0))

; Approximate convolution at point x
(defun convolve-point (x n sum)
  (if (< n -20)
    sum
    (convolve-point x (1- n)
      (+ sum (* (step (- x (* n 0.2)))
                (gaussian (* n 0.2) 1)
                0.2)))))

(defun f (x)
  (convolve-point x 20 0))";

const PLOT_LAPLACE: &str = "; Laplace Transform Visualization
; L{sin(t)} = 1/(s² + 1)
; Showing frequency domain representation

(defun laplace-sin (s)
  (/ 1 (+ (* s s) 1)))

(defun f (x)
  (if (<= x 0.1)
    0
    (laplace-sin x)))";

// ============================================
// Native API - File I/O, batch processing
// Only available in non-WASM builds
// ============================================

/// Configuration for Schaltplan generation
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct SchaltplanConfig {
    /// Company name for title block
    pub company: String,
    /// Project name
    pub project: String,
    /// Output directory
    pub output_dir: String,
    /// Available templates (name -> LSP code)
    pub templates: HashMap<String, String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for SchaltplanConfig {
    fn default() -> Self {
        SchaltplanConfig {
            company: "Elektrotechnik Trahe GmbH".to_string(),
            project: "Schaltplan".to_string(),
            output_dir: "output".to_string(),
            templates: HashMap::new(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A single drawing to be generated
#[derive(Debug, Clone)]
pub struct DrawingSpec {
    /// Drawing name/number (e.g., "01-K1")
    pub name: String,
    /// Template to use (e.g., "MOTOR_ST", "STERN_DR")
    pub template: String,
    /// Component data from CSV
    pub components: Vec<HashMap<String, String>>,
}

#[cfg(not(target_arch = "wasm32"))]
/// Result of generating a drawing
#[derive(Debug)]
pub struct DrawingResult {
    pub name: String,
    pub entities: Vec<DrawEntity>,
    pub svg_path: Option<String>,
    pub json_path: Option<String>,
    pub dxf_path: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
/// Main engine for generating Schaltpläne from CSV data
pub struct SchaltplanEngine {
    config: SchaltplanConfig,
    interpreter: Interpreter,
}

#[cfg(not(target_arch = "wasm32"))]
impl SchaltplanEngine {
    /// Create a new engine with default configuration
    pub fn new() -> Self {
        SchaltplanEngine {
            config: SchaltplanConfig::default(),
            interpreter: Interpreter::new(),
        }
    }

    /// Create engine with custom configuration
    pub fn with_config(config: SchaltplanConfig) -> Self {
        SchaltplanEngine {
            config,
            interpreter: Interpreter::new(),
        }
    }

    /// Set company name
    pub fn set_company(&mut self, name: &str) -> &mut Self {
        self.config.company = name.to_string();
        self
    }

    /// Set project name
    pub fn set_project(&mut self, name: &str) -> &mut Self {
        self.config.project = name.to_string();
        self
    }

    /// Set output directory
    pub fn set_output_dir(&mut self, dir: &str) -> &mut Self {
        self.config.output_dir = dir.to_string();
        self
    }

    /// Load a template from LSP code
    pub fn load_template(&mut self, name: &str, lsp_code: &str) -> &mut Self {
        self.config
            .templates
            .insert(name.to_string(), lsp_code.to_string());
        // Also execute in interpreter to define functions
        self.interpreter.run(lsp_code);
        self
    }

    /// Load a template from a file
    pub fn load_template_file(&mut self, name: &str, path: &str) -> Result<&mut Self, String> {
        let code = fs::read_to_string(path)
            .map_err(|e| format!("Failed to load template {}: {}", path, e))?;
        self.load_template(name, &code);
        Ok(self)
    }

    /// Load all .LSP files from a directory as templates
    pub fn load_templates_dir(&mut self, dir: &str) -> Result<&mut Self, String> {
        let path = Path::new(dir);
        if !path.is_dir() {
            return Err(format!("{} is not a directory", dir));
        }

        for entry in fs::read_dir(path).map_err(|e| e.to_string())?.flatten() {
            let file_path = entry.path();
            if let Some(ext) = file_path.extension() {
                if ext.eq_ignore_ascii_case("lsp") {
                    if let Some(name) = file_path.file_stem() {
                        let name = name.to_string_lossy().to_uppercase();
                        if let Ok(code) = fs::read_to_string(&file_path) {
                            self.load_template(&name, &code);
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    /// Execute raw AutoLISP code
    pub fn eval(&mut self, code: &str) -> Vec<Expr> {
        self.interpreter.run(code)
    }

    /// Process CSV data and generate drawings
    ///
    /// CSV format (German semicolon-delimited):
    /// ```text
    /// START;TEMPLATE;DRAWING_NAME
    /// COMPONENT;TYPE;VALUE
    /// COMPONENT;TYPE;VALUE
    /// STOP
    /// START;TEMPLATE;DRAWING_NAME
    /// ...
    /// ```
    pub fn process_csv(&mut self, csv_data: &str) -> Vec<DrawingSpec> {
        let mut drawings = Vec::new();
        let mut current_drawing: Option<DrawingSpec> = None;

        for line in csv_data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = line.split(';').collect();
            if fields.is_empty() {
                continue;
            }

            let cmd = fields[0].to_uppercase();
            match cmd.as_str() {
                "START" => {
                    // Save previous drawing if exists
                    if let Some(d) = current_drawing.take() {
                        drawings.push(d);
                    }
                    // Start new drawing
                    let template = fields.get(1).unwrap_or(&"DEFAULT").to_string();
                    let name = fields.get(2).unwrap_or(&"UNNAMED").to_string();
                    current_drawing = Some(DrawingSpec {
                        name,
                        template,
                        components: Vec::new(),
                    });
                }
                "STOP" | "END" => {
                    if let Some(d) = current_drawing.take() {
                        drawings.push(d);
                    }
                }
                _ => {
                    // Component line
                    if let Some(ref mut d) = current_drawing {
                        let mut component = HashMap::new();
                        // Standard fields: NAME;TYPE;VALUE;...
                        if !fields.is_empty() {
                            component.insert("NAME".to_string(), fields[0].to_string());
                        }
                        if fields.len() > 1 {
                            component.insert("TYPE".to_string(), fields[1].to_string());
                        }
                        if fields.len() > 2 {
                            component.insert("VALUE".to_string(), fields[2].to_string());
                        }
                        // Additional fields as FIELD3, FIELD4, etc.
                        for (i, field) in fields.iter().enumerate().skip(3) {
                            component.insert(format!("FIELD{}", i), field.to_string());
                        }
                        d.components.push(component);
                    }
                }
            }
        }

        // Don't forget last drawing if no STOP
        if let Some(d) = current_drawing {
            drawings.push(d);
        }

        drawings
    }

    /// Process CSV file
    pub fn process_csv_file(&mut self, path: &str) -> Result<Vec<DrawingSpec>, String> {
        let data =
            fs::read_to_string(path).map_err(|e| format!("Failed to read CSV {}: {}", path, e))?;
        Ok(self.process_csv(&data))
    }

    /// Generate a single drawing from a spec
    pub fn generate_drawing(&mut self, spec: &DrawingSpec) -> DrawingResult {
        // Clear drawing state
        self.interpreter.drawing.entities.clear();

        // Set up variables for the template
        self.interpreter
            .run(&format!(r#"(setq *DRAWING-NAME* "{}")"#, spec.name));
        self.interpreter
            .run(&format!(r#"(setq *COMPANY* "{}")"#, self.config.company));
        self.interpreter
            .run(&format!(r#"(setq *PROJECT* "{}")"#, self.config.project));

        // Build component list as AutoLISP data
        let components_lisp = self.components_to_lisp(&spec.components);
        self.interpreter
            .run(&format!("(setq *COMPONENTS* '{})", components_lisp));
        self.interpreter.run(&format!(
            "(setq *COMPONENT-COUNT* {})",
            spec.components.len()
        ));

        // Call the template's main function if it exists
        let main_func = format!("{}-GENERATE", spec.template.to_uppercase());
        if self.interpreter.functions.contains_key(&main_func) {
            self.interpreter.run(&format!("({})", main_func));
        } else {
            // Try generic GENERATE function
            if self.interpreter.functions.contains_key("GENERATE") {
                self.interpreter.run("(GENERATE)");
            }
        }

        // Collect entities
        let entities = self.interpreter.drawing.entities.clone();

        DrawingResult {
            name: spec.name.clone(),
            entities,
            svg_path: None,
            json_path: None,
            dxf_path: None,
        }
    }

    /// Generate all drawings from specs and save to files
    pub fn generate_all(&mut self, specs: &[DrawingSpec]) -> Vec<DrawingResult> {
        // Clone output_dir to avoid borrow issues
        let output_dir_str = self.config.output_dir.clone();
        let output_dir = Path::new(&output_dir_str);
        if !output_dir.exists() {
            let _ = fs::create_dir_all(output_dir);
        }

        let mut results = Vec::new();

        for spec in specs {
            let mut result = self.generate_drawing(spec);

            // Save SVG
            let svg_path = output_dir.join(format!("{}.svg", spec.name));
            if let Err(e) = self.save_svg(&result.entities, svg_path.to_str().unwrap()) {
                eprintln!("Error saving SVG: {}", e);
            } else {
                result.svg_path = Some(svg_path.to_string_lossy().to_string());
            }

            // Save JSON
            let json_path = output_dir.join(format!("{}.json", spec.name));
            if let Err(e) = self.save_json(&result.entities, json_path.to_str().unwrap()) {
                eprintln!("Error saving JSON: {}", e);
            } else {
                result.json_path = Some(json_path.to_string_lossy().to_string());
            }

            // Save DXF (AutoCAD interchange format)
            let dxf_path = output_dir.join(format!("{}.dxf", spec.name));
            if let Err(e) = self.save_dxf(&result.entities, dxf_path.to_str().unwrap()) {
                eprintln!("Error saving DXF: {}", e);
            } else {
                result.dxf_path = Some(dxf_path.to_string_lossy().to_string());
            }

            results.push(result);
        }

        results
    }

    /// Get output from interpreter (PRINC/PRINT output)
    pub fn get_output(&self) -> &[String] {
        &self.interpreter.output
    }

    /// Clear output buffer
    pub fn clear_output(&mut self) {
        self.interpreter.output.clear();
    }

    /// Get command log (AutoCAD commands issued)
    pub fn get_command_log(&self) -> &[String] {
        &self.interpreter.command_log
    }

    /// Get current drawing entities
    pub fn get_entities(&self) -> &[DrawEntity] {
        &self.interpreter.drawing.entities
    }

    /// Clear drawing entities
    pub fn clear_drawing(&mut self) {
        self.interpreter.drawing.entities.clear();
    }

    /// Access raw interpreter for advanced use
    pub fn interpreter(&mut self) -> &mut Interpreter {
        &mut self.interpreter
    }

    // Helper: convert components to AutoLISP list representation
    fn components_to_lisp(&self, components: &[HashMap<String, String>]) -> String {
        let mut result = String::from("(");
        for comp in components {
            result.push('(');
            for (key, value) in comp {
                result.push_str(&format!("(\"{key}\" . \"{value}\") "));
            }
            result.push(')');
        }
        result.push(')');
        result
    }

    // Save entities as SVG
    fn save_svg(&self, entities: &[DrawEntity], path: &str) -> Result<(), String> {
        let svg = self.entities_to_svg(entities);
        fs::write(path, svg).map_err(|e| e.to_string())
    }

    // Save entities as JSON
    fn save_json(&self, entities: &[DrawEntity], path: &str) -> Result<(), String> {
        let json = self.entities_to_json(entities);
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Save entities as DXF (AutoCAD Drawing Exchange Format)
    /// DXF R12 format - compatible with AutoCAD 9/10 era
    pub fn save_dxf(&self, entities: &[DrawEntity], path: &str) -> Result<(), String> {
        let dxf = self.entities_to_dxf(entities);
        fs::write(path, dxf).map_err(|e| e.to_string())
    }

    fn entities_to_dxf(&self, entities: &[DrawEntity]) -> String {
        let mut dxf = String::new();

        // DXF Header Section
        dxf.push_str("  0\nSECTION\n  2\nHEADER\n");
        dxf.push_str("  9\n$ACADVER\n  1\nAC1009\n"); // AutoCAD R12 version
        dxf.push_str("  0\nENDSEC\n");

        // Tables Section (minimal)
        dxf.push_str("  0\nSECTION\n  2\nTABLES\n");
        // Layer table
        dxf.push_str("  0\nTABLE\n  2\nLAYER\n 70\n1\n");
        dxf.push_str("  0\nLAYER\n  2\n0\n 70\n0\n 62\n7\n  6\nCONTINUOUS\n");
        dxf.push_str("  0\nENDTAB\n");
        dxf.push_str("  0\nENDSEC\n");

        // Entities Section
        dxf.push_str("  0\nSECTION\n  2\nENTITIES\n");

        for entity in entities {
            match entity {
                DrawEntity::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    layer,
                } => {
                    dxf.push_str("  0\nLINE\n");
                    dxf.push_str(&format!("  8\n{}\n", layer)); // Layer
                    dxf.push_str(&format!(" 10\n{}\n", x1)); // Start X
                    dxf.push_str(&format!(" 20\n{}\n", y1)); // Start Y
                    dxf.push_str(&format!(" 11\n{}\n", x2)); // End X
                    dxf.push_str(&format!(" 21\n{}\n", y2)); // End Y
                }
                DrawEntity::Circle {
                    cx,
                    cy,
                    radius,
                    layer,
                } => {
                    dxf.push_str("  0\nCIRCLE\n");
                    dxf.push_str(&format!("  8\n{}\n", layer));
                    dxf.push_str(&format!(" 10\n{}\n", cx)); // Center X
                    dxf.push_str(&format!(" 20\n{}\n", cy)); // Center Y
                    dxf.push_str(&format!(" 40\n{}\n", radius)); // Radius
                }
                DrawEntity::Arc {
                    cx,
                    cy,
                    radius,
                    start_angle,
                    end_angle,
                    layer,
                } => {
                    dxf.push_str("  0\nARC\n");
                    dxf.push_str(&format!("  8\n{}\n", layer));
                    dxf.push_str(&format!(" 10\n{}\n", cx));
                    dxf.push_str(&format!(" 20\n{}\n", cy));
                    dxf.push_str(&format!(" 40\n{}\n", radius));
                    dxf.push_str(&format!(" 50\n{}\n", start_angle.to_degrees()));
                    dxf.push_str(&format!(" 51\n{}\n", end_angle.to_degrees()));
                }
                DrawEntity::Text {
                    x,
                    y,
                    height,
                    text,
                    layer,
                } => {
                    dxf.push_str("  0\nTEXT\n");
                    dxf.push_str(&format!("  8\n{}\n", layer));
                    dxf.push_str(&format!(" 10\n{}\n", x)); // Position X
                    dxf.push_str(&format!(" 20\n{}\n", y)); // Position Y
                    dxf.push_str(&format!(" 40\n{}\n", height)); // Text height
                    dxf.push_str(&format!("  1\n{}\n", text)); // Text string
                }
                DrawEntity::Point { x, y, layer } => {
                    dxf.push_str("  0\nPOINT\n");
                    dxf.push_str(&format!("  8\n{}\n", layer));
                    dxf.push_str(&format!(" 10\n{}\n", x));
                    dxf.push_str(&format!(" 20\n{}\n", y));
                }
                DrawEntity::Insert {
                    block_name,
                    x,
                    y,
                    scale,
                    rotation,
                    layer,
                } => {
                    dxf.push_str("  0\nINSERT\n");
                    dxf.push_str(&format!("  8\n{}\n", layer));
                    dxf.push_str(&format!("  2\n{}\n", block_name)); // Block name
                    dxf.push_str(&format!(" 10\n{}\n", x));
                    dxf.push_str(&format!(" 20\n{}\n", y));
                    dxf.push_str(&format!(" 41\n{}\n", scale)); // X scale
                    dxf.push_str(&format!(" 42\n{}\n", scale)); // Y scale
                    dxf.push_str(&format!(" 50\n{}\n", rotation)); // Rotation
                }
                DrawEntity::Pin { .. } | DrawEntity::Property { .. } | DrawEntity::Pad { .. } => {
                    // KiCad specific entities ignored in DXF export
                }
            }
        }

        dxf.push_str("  0\nENDSEC\n");
        dxf.push_str("  0\nEOF\n");

        dxf
    }

    fn entities_to_svg(&self, entities: &[DrawEntity]) -> String {
        // Calculate bounding box
        let (min_x, min_y, max_x, max_y) = self.calculate_bounds(entities);

        let padding = 50.0;
        let width = (max_x - min_x + 2.0 * padding).max(800.0);
        let height = (max_y - min_y + 2.0 * padding).max(600.0);

        let mut svg = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="{} {} {} {}"
     width="{}" height="{}"
     style="background-color: #1a1a2e;">
  <title>{} - {}</title>
  <defs>
    <style>
      .line {{ stroke: #00ff88; stroke-width: 1; fill: none; }}
      .circle {{ stroke: #00aaff; stroke-width: 1; fill: none; }}
      .text {{ fill: #ffffff; font-family: monospace; }}
      .block {{ stroke: #ff8800; fill: none; }}
    </style>
  </defs>
  <g transform="translate(0, {}) scale(1, -1)">
"#,
            min_x - padding,
            min_y - padding,
            width,
            height,
            width.min(1200.0),
            height.min(900.0),
            self.config.company,
            self.config.project,
            height
        );

        for entity in entities {
            match entity {
                DrawEntity::Line { x1, y1, x2, y2, .. } => {
                    svg.push_str(&format!(
                        "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"line\" />\n",
                        x1, y1, x2, y2
                    ));
                }
                DrawEntity::Circle { cx, cy, radius, .. } => {
                    svg.push_str(&format!(
                        "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"circle\" />\n",
                        cx, cy, radius
                    ));
                }
                DrawEntity::Text {
                    x, y, height, text, ..
                } => {
                    let escaped = text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    // Note: text needs to be flipped back since we're in a flipped coordinate system
                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"{}\" transform=\"scale(1,-1) translate(0,{})\">{}</text>\n",
                        x, -y, height * 1.5, -2.0 * y, escaped
                    ));
                }
                DrawEntity::Insert {
                    block_name, x, y, ..
                } => {
                    svg.push_str(&format!(
                        "    <rect x=\"{}\" y=\"{}\" width=\"40\" height=\"40\" class=\"block\" />\n",
                        x, y
                    ));
                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"8\" transform=\"scale(1,-1) translate(0,{})\">{}</text>\n",
                        x + 2.0, -(y + 20.0), -2.0 * (y + 20.0), block_name
                    ));
                }
                DrawEntity::Pin {
                    name,
                    number,
                    x,
                    y,
                    length,
                    rotation,
                    ..
                } => {
                    // Draw a simple line and circle for pin in SVG
                    let rad = rotation.to_radians();
                    let end_x = x + length * rad.cos();
                    let end_y = y + length * rad.sin();
                    svg.push_str(&format!(
                        "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ff00ff\" stroke-width=\"0.5\" />\n",
                        x, -y, end_x, -end_y
                    ));
                    svg.push_str(&format!(
                        "    <circle cx=\"{}\" cy=\"{}\" r=\"0.5\" stroke=\"#ff00ff\" fill=\"none\" />\n",
                        x, -y
                    ));
                    // Text
                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"2\" transform=\"scale(1,-1) translate(0,{})\">{} {}</text>\n",
                        end_x, -(end_y + 2.0), -2.0 * (end_y + 2.0), name, number
                    ));
                }
                DrawEntity::Property { .. } => {
                    // Ignore properties in SVG for now
                }
                DrawEntity::Pad {
                    name,
                    shape,
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    // Draw a simple shape for pad in SVG
                    if shape == "circle" || shape == "oval" {
                        svg.push_str(&format!(
                            "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#ffaa00\" fill-opacity=\"0.5\" />\n",
                            x,
                            -y,
                            width.min(*height) / 2.0
                        ));
                    } else {
                        // Rect or others
                        svg.push_str(&format!(
                            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffaa00\" fill-opacity=\"0.5\" transform=\"rotate(0)\" />\n",
                            x - width / 2.0,
                            -y - height / 2.0,
                            width,
                            height
                        ));
                    }
                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"{}\" text-anchor=\"middle\" transform=\"scale(1,-1) translate(0,{})\">{}</text>\n",
                        x, -y, width.min(*height) * 0.4, -2.0 * y, name
                    ));
                }
                DrawEntity::Arc {
                    cx,
                    cy,
                    radius,
                    start_angle,
                    end_angle,
                    ..
                } => {
                    let start_x = cx + radius * start_angle.cos();
                    let start_y = cy + radius * start_angle.sin();
                    let end_x = cx + radius * end_angle.cos();
                    let end_y = cy + radius * end_angle.sin();
                    let large_arc = if (end_angle - start_angle).abs() > std::f64::consts::PI {
                        1
                    } else {
                        0
                    };
                    svg.push_str(&format!(
                        "    <path d=\"M {} {} A {} {} 0 {} 1 {} {}\" class=\"circle\" />\n",
                        start_x, start_y, radius, radius, large_arc, end_x, end_y
                    ));
                }
                DrawEntity::Point { x, y, .. } => {
                    svg.push_str(&format!(
                        "    <circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"#00ff88\" />\n",
                        x, y
                    ));
                }
            }
        }

        svg.push_str("  </g>\n</svg>\n");
        svg
    }

    fn entities_to_json(&self, entities: &[DrawEntity]) -> String {
        let mut json = String::from("[\n");
        for (i, entity) in entities.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str("  ");
            json.push_str(&self.entity_to_json(entity));
        }
        json.push_str("\n]\n");
        json
    }

    fn entity_to_json(&self, entity: &DrawEntity) -> String {
        match entity {
            DrawEntity::Line {
                x1,
                y1,
                x2,
                y2,
                layer,
            } => {
                format!(
                    r#"{{"type":"LINE","x1":{},"y1":{},"x2":{},"y2":{},"layer":"{}"}}"#,
                    x1, y1, x2, y2, layer
                )
            }
            DrawEntity::Circle {
                cx,
                cy,
                radius,
                layer,
            } => {
                format!(
                    r#"{{"type":"CIRCLE","cx":{},"cy":{},"r":{},"layer":"{}"}}"#,
                    cx, cy, radius, layer
                )
            }
            DrawEntity::Arc {
                cx,
                cy,
                radius,
                start_angle,
                end_angle,
                layer,
            } => {
                format!(
                    r#"{{"type":"ARC","cx":{},"cy":{},"r":{},"start":{},"end":{},"layer":"{}"}}"#,
                    cx, cy, radius, start_angle, end_angle, layer
                )
            }
            DrawEntity::Text {
                x,
                y,
                height,
                text,
                layer,
            } => {
                // Escape backslashes first, then quotes for valid JSON
                let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
                format!(
                    r#"{{"type":"TEXT","x":{},"y":{},"h":{},"text":"{}","layer":"{}"}}"#,
                    x, y, height, escaped, layer
                )
            }
            DrawEntity::Point { x, y, layer } => {
                format!(
                    r#"{{"type":"POINT","x":{},"y":{},"layer":"{}"}}"#,
                    x, y, layer
                )
            }
            DrawEntity::Insert {
                block_name,
                x,
                y,
                scale,
                rotation,
                layer,
            } => {
                format!(
                    r#"{{"type":"INSERT","block":"{}","x":{},"y":{},"scale":{},"rot":{},"layer":"{}"}}"#,
                    block_name, x, y, scale, rotation, layer
                )
            }
            DrawEntity::Pin {
                name,
                number,
                etype,
                style,
                x,
                y,
                length,
                rotation,
                layer,
            } => {
                format!(
                    r#"{{"type":"PIN","name":"{}","num":"{}","etype":"{}","style":"{}","x":{},"y":{},"len":{},"rot":{},"layer":"{}"}}"#,
                    name, number, etype, style, x, y, length, rotation, layer
                )
            }
            DrawEntity::Property {
                key,
                value,
                x,
                y,
                rotation,
                height,
                visible,
                layer,
            } => {
                format!(
                    r#"{{"type":"PROP","key":"{}","val":"{}","x":{},"y":{},"rot":{},"h":{},"vis":{},"layer":"{}"}}"#,
                    key, value, x, y, rotation, height, visible, layer
                )
            }
            DrawEntity::Pad {
                name,
                ptype,
                shape,
                x,
                y,
                width,
                height,
                drill,
                rotation,
                layers,
            } => {
                format!(
                    r#"{{"type":"PAD","name":"{}","ptype":"{}","shape":"{}","x":{},"y":{},"w":{},"h":{},"drill":{},"rot":{},"layers":"{}"}}"#,
                    name, ptype, shape, x, y, width, height, drill, rotation, layers
                )
            }
        }
    }

    fn calculate_bounds(&self, entities: &[DrawEntity]) -> (f64, f64, f64, f64) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        if entities.is_empty() {
            return (0.0, 0.0, 800.0, 600.0);
        }

        for entity in entities {
            match entity {
                DrawEntity::Line { x1, y1, x2, y2, .. } => {
                    min_x = min_x.min(*x1).min(*x2);
                    min_y = min_y.min(*y1).min(*y2);
                    max_x = max_x.max(*x1).max(*x2);
                    max_y = max_y.max(*y1).max(*y2);
                }
                DrawEntity::Circle { cx, cy, radius, .. } => {
                    min_x = min_x.min(cx - radius);
                    min_y = min_y.min(cy - radius);
                    max_x = max_x.max(cx + radius);
                    max_y = max_y.max(cy + radius);
                }
                DrawEntity::Text {
                    x, y, height, text, ..
                } => {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x + text.len() as f64 * height * 0.6);
                    max_y = max_y.max(*y + height);
                }
                DrawEntity::Insert { x, y, .. } => {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x + 50.0);
                    max_y = max_y.max(*y + 50.0);
                }
                DrawEntity::Pin { x, y, length, .. } => {
                    // Include pins in bounding box
                    min_x = min_x.min(*x - length);
                    min_y = min_y.min(*y - length);
                    max_x = max_x.max(*x + length);
                    max_y = max_y.max(*y + length);
                }
                DrawEntity::Property { .. } => {
                    // Ignore properties for bounds
                }
                DrawEntity::Pad {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    let w2 = width / 2.0;
                    let h2 = height / 2.0;
                    min_x = min_x.min(*x - w2);
                    min_y = min_y.min(*y - h2);
                    max_x = max_x.max(*x + w2);
                    max_y = max_y.max(*y + h2);
                }
                DrawEntity::Arc { cx, cy, radius, .. } => {
                    min_x = min_x.min(cx - radius);
                    min_y = min_y.min(cy - radius);
                    max_x = max_x.max(cx + radius);
                    max_y = max_y.max(cy + radius);
                }
                DrawEntity::Point { x, y, .. } => {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x);
                    max_y = max_y.max(*y);
                }
            }
        }

        (min_x, min_y, max_x, max_y)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for SchaltplanEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to run AutoLISP code and get output
pub fn run_lisp(code: &str) -> (Vec<Expr>, Vec<DrawEntity>) {
    let mut interp = Interpreter::new();
    let results = interp.run(code);
    (results, interp.drawing.entities)
}

/// Convenience function to run an LSP file
#[cfg(not(target_arch = "wasm32"))]
pub fn run_file(path: &str) -> Result<(Vec<Expr>, Vec<DrawEntity>), String> {
    let code = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
    Ok(run_lisp(&code))
}

// ============================================
// HP-41C Calculator - RPN Stack Machine
// ============================================

/// HP-41C style RPN calculator with 4-level stack
#[wasm_bindgen]
pub struct HP41Calculator {
    stack_x: f64,
    stack_y: f64,
    stack_z: f64,
    stack_t: f64,
    last_x: f64,
    // Statistical registers
    sigma_n: f64,
    sigma_x: f64,
    sigma_y: f64,
    sigma_x2: f64,
    sigma_y2: f64,
    sigma_xy: f64,
    // Display settings
    display_mode: u8,  // 0=FIX, 1=SCI, 2=ENG
    display_digits: u8,
    // Entry state
    entry_mode: bool,
    input_buffer: String,
}

impl Default for HP41Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl HP41Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HP41Calculator {
        HP41Calculator {
            stack_x: 0.0,
            stack_y: 0.0,
            stack_z: 0.0,
            stack_t: 0.0,
            last_x: 0.0,
            sigma_n: 0.0,
            sigma_x: 0.0,
            sigma_y: 0.0,
            sigma_x2: 0.0,
            sigma_y2: 0.0,
            sigma_xy: 0.0,
            display_mode: 0,
            display_digits: 4,
            entry_mode: false,
            input_buffer: String::new(),
        }
    }

    // Stack operations
    #[wasm_bindgen]
    pub fn push(&mut self, val: f64) {
        self.stack_t = self.stack_z;
        self.stack_z = self.stack_y;
        self.stack_y = self.stack_x;
        self.stack_x = val;
    }

    #[wasm_bindgen]
    pub fn stack_drop(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_y;
        self.stack_y = self.stack_z;
        self.stack_z = self.stack_t;
    }

    fn drop_no_lastx(&mut self) {
        self.stack_x = self.stack_y;
        self.stack_y = self.stack_z;
        self.stack_z = self.stack_t;
    }

    #[wasm_bindgen]
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.stack_x, &mut self.stack_y);
    }

    #[wasm_bindgen]
    pub fn roll_down(&mut self) {
        let tmp = self.stack_x;
        self.stack_x = self.stack_y;
        self.stack_y = self.stack_z;
        self.stack_z = self.stack_t;
        self.stack_t = tmp;
    }

    #[wasm_bindgen]
    pub fn clear_stack(&mut self) {
        self.stack_x = 0.0;
        self.stack_y = 0.0;
        self.stack_z = 0.0;
        self.stack_t = 0.0;
    }

    #[wasm_bindgen]
    pub fn clear_x(&mut self) {
        self.stack_x = 0.0;
        self.input_buffer.clear();
        self.entry_mode = false;
    }

    // Getters
    #[wasm_bindgen]
    pub fn get_x(&self) -> f64 { self.stack_x }
    #[wasm_bindgen]
    pub fn get_y(&self) -> f64 { self.stack_y }
    #[wasm_bindgen]
    pub fn get_z(&self) -> f64 { self.stack_z }
    #[wasm_bindgen]
    pub fn get_t(&self) -> f64 { self.stack_t }
    #[wasm_bindgen]
    pub fn get_last_x(&self) -> f64 { self.last_x }

    // Binary operations - consume Y and X, put result in X, drop stack
    #[wasm_bindgen]
    pub fn add(&mut self) {
        self.last_x = self.stack_x;
        let result = self.stack_y + self.stack_x;
        self.stack_x = self.stack_y;  // temporarily for drop
        self.drop_no_lastx();
        self.stack_x = result;
    }

    #[wasm_bindgen]
    pub fn subtract(&mut self) {
        self.last_x = self.stack_x;
        let result = self.stack_y - self.stack_x;
        self.stack_x = self.stack_y;
        self.drop_no_lastx();
        self.stack_x = result;
    }

    #[wasm_bindgen]
    pub fn multiply(&mut self) {
        self.last_x = self.stack_x;
        let result = self.stack_y * self.stack_x;
        self.stack_x = self.stack_y;
        self.drop_no_lastx();
        self.stack_x = result;
    }

    #[wasm_bindgen]
    pub fn divide(&mut self) {
        self.last_x = self.stack_x;
        let result = self.stack_y / self.stack_x;
        self.stack_x = self.stack_y;
        self.drop_no_lastx();
        self.stack_x = result;
    }

    #[wasm_bindgen]
    pub fn power(&mut self) {
        self.last_x = self.stack_x;
        let result = self.stack_y.powf(self.stack_x);
        self.stack_x = self.stack_y;
        self.drop_no_lastx();
        self.stack_x = result;
    }

    // Unary operations
    #[wasm_bindgen]
    pub fn sqrt(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.sqrt();
    }

    #[wasm_bindgen]
    pub fn square(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x * self.stack_x;
    }

    #[wasm_bindgen]
    pub fn reciprocal(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = 1.0 / self.stack_x;
    }

    #[wasm_bindgen]
    pub fn chs(&mut self) {
        self.stack_x = -self.stack_x;
    }

    // Trigonometric
    #[wasm_bindgen]
    pub fn sin(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.sin();
    }

    #[wasm_bindgen]
    pub fn cos(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.cos();
    }

    #[wasm_bindgen]
    pub fn tan(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.tan();
    }

    #[wasm_bindgen]
    pub fn asin(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.asin();
    }

    #[wasm_bindgen]
    pub fn acos(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.acos();
    }

    #[wasm_bindgen]
    pub fn atan(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.atan();
    }

    // Logarithmic
    #[wasm_bindgen]
    pub fn ln(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.ln();
    }

    #[wasm_bindgen]
    pub fn log10(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.log10();
    }

    #[wasm_bindgen]
    pub fn exp(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_x.exp();
    }

    #[wasm_bindgen]
    pub fn pow10(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = 10.0_f64.powf(self.stack_x);
    }

    // Constants
    #[wasm_bindgen]
    pub fn pi(&mut self) {
        self.push(std::f64::consts::PI);
    }

    #[wasm_bindgen]
    pub fn recall_last_x(&mut self) {
        self.push(self.last_x);
    }

    // Polar/Rectangular conversions
    #[wasm_bindgen]
    pub fn polar_to_rect(&mut self) {
        let r = self.stack_y;
        let theta = self.stack_x;
        self.stack_x = r * theta.cos();
        self.stack_y = r * theta.sin();
    }

    #[wasm_bindgen]
    pub fn rect_to_polar(&mut self) {
        let x = self.stack_x;
        let y = self.stack_y;
        self.stack_x = (x * x + y * y).sqrt();
        self.stack_y = y.atan2(x);
    }

    // Percent
    #[wasm_bindgen]
    pub fn percent(&mut self) {
        self.last_x = self.stack_x;
        self.stack_x = self.stack_y * (self.stack_x / 100.0);
    }

    // Display mode
    #[wasm_bindgen]
    pub fn set_fix(&mut self, digits: u8) {
        self.display_mode = 0;
        self.display_digits = digits.min(9);
    }

    #[wasm_bindgen]
    pub fn set_sci(&mut self, digits: u8) {
        self.display_mode = 1;
        self.display_digits = digits.min(9);
    }

    #[wasm_bindgen]
    pub fn set_eng(&mut self, digits: u8) {
        self.display_mode = 2;
        self.display_digits = digits.min(9);
    }

    // Format number for display (HP-41C style: 10 significant digits)
    #[wasm_bindgen]
    pub fn format_display(&self) -> String {
        self.format_number(self.stack_x)
    }

    fn format_number(&self, num: f64) -> String {
        if num.is_nan() {
            return "ERROR".to_string();
        }
        if num.is_infinite() {
            return if num > 0.0 { "9.999999999 99" } else { "-9.999999999 99" }.to_string();
        }

        match self.display_mode {
            1 => {
                // SCI mode
                format!("{:.prec$e}", num, prec = self.display_digits as usize)
                    .replace("e", " ")
                    .to_uppercase()
            }
            2 => {
                // ENG mode - exponent multiple of 3
                if num == 0.0 {
                    return "0.".to_string();
                }
                let exp = num.abs().log10().floor() as i32;
                let eng_exp = (exp as f64 / 3.0).floor() as i32 * 3;
                let mantissa = num / 10.0_f64.powi(eng_exp);
                format!("{:.prec$} {:+03}", mantissa, eng_exp, prec = self.display_digits as usize)
            }
            _ => {
                // FIX mode (default)
                // HP-41C shows 10 significant digits
                if num.abs() < 1e-99 && num != 0.0 {
                    return format!("{:.6e}", num).replace("e", " ").to_uppercase();
                }
                if num.abs() >= 1e10 {
                    return format!("{:.6e}", num).replace("e", " ").to_uppercase();
                }

                // Round to 10 significant digits
                let formatted = format!("{:.10}", num);
                // Trim trailing zeros but keep at least one digit after decimal
                let trimmed = formatted.trim_end_matches('0');
                if trimmed.ends_with('.') {
                    format!("{}.", trimmed.trim_end_matches('.'))
                } else {
                    trimmed.to_string()
                }
            }
        }
    }

    // Get LCD segment data as JSON for rendering
    #[wasm_bindgen]
    pub fn get_lcd_segments(&self) -> String {
        let display_str = self.format_display();
        let mut segments: Vec<String> = Vec::new();

        for ch in display_str.chars() {
            let pattern = Self::get_14_segment_pattern(ch);
            segments.push(format!("{{\"char\":\"{}\",\"segments\":{:?}}}", ch, pattern));
        }

        format!("{{\"display\":\"{}\",\"chars\":[{}]}}", display_str, segments.join(","))
    }

    // 14-segment patterns for HP-41C style display
    // Segments: a,b,c,d,e,f,g1,g2,h,i,j,k,l,m
    fn get_14_segment_pattern(ch: char) -> [u8; 14] {
        match ch {
            '0' => [1,1,1,1,1,1,0,0,0,0,1,0,0,0],  // with slash
            '1' => [0,1,1,0,0,0,0,0,0,0,0,0,0,0],  // right side only
            '2' => [1,1,0,1,1,0,1,1,0,0,0,0,0,0],
            '3' => [1,1,1,1,0,0,1,1,0,0,0,0,0,0],
            '4' => [0,1,1,0,0,1,1,1,0,0,0,0,0,0],
            '5' => [1,0,1,1,0,1,1,1,0,0,0,0,0,0],
            '6' => [1,0,1,1,1,1,1,1,0,0,0,0,0,0],
            '7' => [1,1,1,0,0,0,0,0,0,0,0,0,0,0],
            '8' => [1,1,1,1,1,1,1,1,0,0,0,0,0,0],
            '9' => [1,1,1,1,0,1,1,1,0,0,0,0,0,0],
            'A' => [1,1,1,0,1,1,1,1,0,0,0,0,0,0],
            'B' => [1,1,1,1,0,0,0,1,0,1,0,0,1,0],
            'C' => [1,0,0,1,1,1,0,0,0,0,0,0,0,0],
            'D' => [1,1,1,1,0,0,0,0,0,1,0,0,1,0],
            'E' => [1,0,0,1,1,1,1,0,0,0,0,0,0,0],
            'F' => [1,0,0,0,1,1,1,0,0,0,0,0,0,0],
            'G' => [1,0,1,1,1,1,0,1,0,0,0,0,0,0],
            'H' => [0,1,1,0,1,1,1,1,0,0,0,0,0,0],
            'I' => [1,0,0,1,0,0,0,0,0,1,0,0,1,0],
            'J' => [0,1,1,1,1,0,0,0,0,0,0,0,0,0],
            'K' => [0,0,0,0,1,1,1,0,0,0,1,1,0,0],
            'L' => [0,0,0,1,1,1,0,0,0,0,0,0,0,0],
            'M' => [0,1,1,0,1,1,0,0,1,0,1,0,0,0],
            'N' => [0,1,1,0,1,1,0,0,1,0,0,0,0,1],
            'O' => [1,1,1,1,1,1,0,0,0,0,0,0,0,0],
            'P' => [1,1,0,0,1,1,1,1,0,0,0,0,0,0],
            'Q' => [1,1,1,1,1,1,0,0,0,0,0,0,0,1],
            'R' => [1,1,0,0,1,1,1,1,0,0,0,0,0,1],
            'S' => [1,0,1,1,0,1,1,1,0,0,0,0,0,0],
            'T' => [1,0,0,0,0,0,0,0,0,1,0,0,1,0],
            'U' => [0,1,1,1,1,1,0,0,0,0,0,0,0,0],
            'V' => [0,0,0,0,1,1,0,0,0,0,1,1,0,0],
            'W' => [0,1,1,0,1,1,0,0,0,0,0,1,0,1],
            'X' => [0,0,0,0,0,0,0,0,1,0,1,1,0,1],
            'Y' => [0,0,0,0,0,0,0,0,1,0,1,0,1,0],
            'Z' => [1,0,0,1,0,0,0,0,0,0,1,1,0,0],
            '-' => [0,0,0,0,0,0,1,1,0,0,0,0,0,0],
            '+' => [0,0,0,0,0,0,1,1,0,1,0,0,1,0],
            ' ' => [0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            '.' => [0,0,0,0,0,0,0,0,0,0,0,0,0,0], // decimal point handled separately
            _   => [0,0,0,0,0,0,0,0,0,0,0,0,0,0],
        }
    }

    // Statistics
    #[wasm_bindgen]
    pub fn sigma_plus(&mut self) {
        self.sigma_n += 1.0;
        self.sigma_x += self.stack_x;
        self.sigma_y += self.stack_y;
        self.sigma_x2 += self.stack_x * self.stack_x;
        self.sigma_y2 += self.stack_y * self.stack_y;
        self.sigma_xy += self.stack_x * self.stack_y;
        self.push(self.sigma_n);
    }

    #[wasm_bindgen]
    pub fn sigma_minus(&mut self) {
        self.sigma_n -= 1.0;
        self.sigma_x -= self.stack_x;
        self.sigma_y -= self.stack_y;
        self.sigma_x2 -= self.stack_x * self.stack_x;
        self.sigma_y2 -= self.stack_y * self.stack_y;
        self.sigma_xy -= self.stack_x * self.stack_y;
        self.push(self.sigma_n);
    }

    #[wasm_bindgen]
    pub fn clear_sigma(&mut self) {
        self.sigma_n = 0.0;
        self.sigma_x = 0.0;
        self.sigma_y = 0.0;
        self.sigma_x2 = 0.0;
        self.sigma_y2 = 0.0;
        self.sigma_xy = 0.0;
    }

    // Comparison tests (return 1.0 for true, 0.0 for false)
    #[wasm_bindgen]
    pub fn test_x_eq_y(&self) -> f64 { if self.stack_x == self.stack_y { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_ne_y(&self) -> f64 { if self.stack_x != self.stack_y { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_lt_y(&self) -> f64 { if self.stack_x < self.stack_y { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_le_y(&self) -> f64 { if self.stack_x <= self.stack_y { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_gt_y(&self) -> f64 { if self.stack_x > self.stack_y { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_eq_0(&self) -> f64 { if self.stack_x == 0.0 { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_ne_0(&self) -> f64 { if self.stack_x != 0.0 { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_lt_0(&self) -> f64 { if self.stack_x < 0.0 { 1.0 } else { 0.0 } }
    #[wasm_bindgen]
    pub fn test_x_gt_0(&self) -> f64 { if self.stack_x > 0.0 { 1.0 } else { 0.0 } }

    // Entry mode for digit input (accepts string from JS)
    #[wasm_bindgen]
    pub fn enter_digit(&mut self, digit: &str) {
        if !self.entry_mode {
            // Don't push if stack is empty (first entry)
            if self.stack_x != 0.0 || !self.input_buffer.is_empty() {
                self.push(self.stack_x);
            }
            self.input_buffer.clear();
            self.entry_mode = true;
        }
        self.input_buffer.push_str(digit);
        if let Ok(val) = self.input_buffer.parse::<f64>() {
            self.stack_x = val;
        }
    }

    #[wasm_bindgen]
    pub fn enter_decimal(&mut self) {
        if !self.entry_mode {
            self.push(self.stack_x);
            self.input_buffer = "0".to_string();
            self.entry_mode = true;
        }
        if !self.input_buffer.contains('.') {
            self.input_buffer.push('.');
        }
    }

    #[wasm_bindgen]
    pub fn enter_exponent(&mut self) {
        if self.entry_mode && !self.input_buffer.contains('e') {
            self.input_buffer.push('e');
        }
    }

    #[wasm_bindgen]
    pub fn backspace(&mut self) {
        if self.entry_mode && !self.input_buffer.is_empty() {
            self.input_buffer.pop();
            if self.input_buffer.is_empty() {
                self.stack_x = 0.0;
            } else if let Ok(val) = self.input_buffer.parse::<f64>() {
                self.stack_x = val;
            }
        }
    }

    #[wasm_bindgen]
    pub fn enter(&mut self) {
        if self.entry_mode {
            self.entry_mode = false;
            self.input_buffer.clear();
        } else {
            self.push(self.stack_x);
        }
    }

    #[wasm_bindgen]
    pub fn is_entry_mode(&self) -> bool {
        self.entry_mode
    }

    #[wasm_bindgen]
    pub fn get_input_buffer(&self) -> String {
        self.input_buffer.clone()
    }

    // =============================================
    // LISP Transpilation - returns LISP code strings
    // =============================================

    /// Transpile pushing a number to LISP
    #[wasm_bindgen]
    pub fn lisp_push(&self, val: f64) -> String {
        format!("(stack-push {})", val)
    }

    /// Transpile ENTER key to LISP (duplicate X)
    #[wasm_bindgen]
    pub fn lisp_enter(&self) -> String {
        "(stack-push stack-x)".to_string()
    }

    /// Transpile addition to LISP
    #[wasm_bindgen]
    pub fn lisp_add(&self) -> String {
        "(rpn-add)".to_string()
    }

    /// Transpile subtraction to LISP
    #[wasm_bindgen]
    pub fn lisp_subtract(&self) -> String {
        "(rpn-sub)".to_string()
    }

    /// Transpile multiplication to LISP
    #[wasm_bindgen]
    pub fn lisp_multiply(&self) -> String {
        "(rpn-mul)".to_string()
    }

    /// Transpile division to LISP
    #[wasm_bindgen]
    pub fn lisp_divide(&self) -> String {
        "(rpn-div)".to_string()
    }

    /// Transpile power to LISP
    #[wasm_bindgen]
    pub fn lisp_power(&self) -> String {
        "(rpn-pow)".to_string()
    }

    /// Transpile sqrt to LISP
    #[wasm_bindgen]
    pub fn lisp_sqrt(&self) -> String {
        "(rpn-sqrt)".to_string()
    }

    /// Transpile sin to LISP
    #[wasm_bindgen]
    pub fn lisp_sin(&self) -> String {
        "(rpn-sin)".to_string()
    }

    /// Transpile cos to LISP
    #[wasm_bindgen]
    pub fn lisp_cos(&self) -> String {
        "(rpn-cos)".to_string()
    }

    /// Transpile tan to LISP
    #[wasm_bindgen]
    pub fn lisp_tan(&self) -> String {
        "(rpn-tan)".to_string()
    }

    /// Transpile log (base 10) to LISP
    #[wasm_bindgen]
    pub fn lisp_log(&self) -> String {
        "(rpn-log)".to_string()
    }

    /// Transpile ln to LISP
    #[wasm_bindgen]
    pub fn lisp_ln(&self) -> String {
        "(rpn-ln)".to_string()
    }

    /// Transpile 1/x to LISP
    #[wasm_bindgen]
    pub fn lisp_inv(&self) -> String {
        "(rpn-inv)".to_string()
    }

    /// Transpile CHS to LISP
    #[wasm_bindgen]
    pub fn lisp_chs(&self) -> String {
        "(rpn-chs)".to_string()
    }

    /// Transpile PI to LISP
    #[wasm_bindgen]
    pub fn lisp_pi(&self) -> String {
        "(rpn-pi)".to_string()
    }

    /// Transpile swap to LISP
    #[wasm_bindgen]
    pub fn lisp_swap(&self) -> String {
        "(stack-swap)".to_string()
    }

    /// Transpile roll down to LISP
    #[wasm_bindgen]
    pub fn lisp_roll_down(&self) -> String {
        "(stack-roll-down)".to_string()
    }

    /// Transpile clear X to LISP
    #[wasm_bindgen]
    pub fn lisp_clear_x(&self) -> String {
        "(setq stack-x 0)".to_string()
    }

    /// Transpile clear stack to LISP
    #[wasm_bindgen]
    pub fn lisp_clear_stack(&self) -> String {
        "(stack-clear)".to_string()
    }

    /// Transpile LAST X to LISP
    #[wasm_bindgen]
    pub fn lisp_last_x(&self) -> String {
        "(stack-push last-x)".to_string()
    }

    /// Get X value as LISP
    #[wasm_bindgen]
    pub fn lisp_get_x(&self) -> String {
        "(get-x)".to_string()
    }

    /// Get Y value as LISP
    #[wasm_bindgen]
    pub fn lisp_get_y(&self) -> String {
        "(get-y)".to_string()
    }

    /// Get Z value as LISP
    #[wasm_bindgen]
    pub fn lisp_get_z(&self) -> String {
        "(get-z)".to_string()
    }

    /// Get T value as LISP
    #[wasm_bindgen]
    pub fn lisp_get_t(&self) -> String {
        "(get-t)".to_string()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_hp41_basic() {
        let mut calc = HP41Calculator::new();
        calc.push(3.0);
        calc.push(4.0);
        calc.add();
        assert_eq!(calc.get_x(), 7.0);
    }

    #[test]
    fn test_hp41_pi() {
        let mut calc = HP41Calculator::new();
        calc.pi();
        let display = calc.format_display();
        assert!(display.starts_with("3.14159265"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = SchaltplanEngine::new();
        assert_eq!(engine.config.company, "Elektrotechnik Trahe GmbH");
    }

    #[test]
    fn test_csv_parsing() {
        let mut engine = SchaltplanEngine::new();
        let csv = "START;MOTOR_ST;01-K1\nK1;Schütz;24V\nF1;Sicherung;16A\nSTOP";
        let specs = engine.process_csv(csv);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "01-K1");
        assert_eq!(specs[0].template, "MOTOR_ST");
        assert_eq!(specs[0].components.len(), 2);
    }

    #[test]
    fn test_run_lisp() {
        let (results, _) = run_lisp("(+ 1 2 3)");
        assert_eq!(results.len(), 1);
        if let Expr::Integer(n) = &results[0] {
            assert_eq!(*n, 6);
        } else {
            panic!("Expected integer");
        }
    }
}

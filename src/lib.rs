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

pub mod interpreter;
pub mod lexer;
pub mod parser;

pub use interpreter::{DrawEntity, DrawingState, Interpreter};
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
        entities_to_svg(&self.interpreter.drawing.entities)
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

    /// Clear the drawing
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.interpreter.drawing.entities.clear();
        self.interpreter.output.clear();
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
        r#"["hello","math","box","spiral","schaltplan","fractal"]"#.to_string()
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
// LISP Examples - stored in Rust
// ============================================

const EXAMPLE_HELLO: &str = r#"; Hello World in AutoLISP
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

const EXAMPLE_MATH: &str = r#"; Math and Recursion
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

const EXAMPLE_BOX: &str = r#"; Draw a parametric box
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

const EXAMPLE_SPIRAL: &str = r#"; Draw a spiral using math
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

const EXAMPLE_SCHALTPLAN: &str = r#"; Mini Schaltplan
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

const EXAMPLE_FRACTAL: &str = r#"; Recursive Tree (simple fractal)
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
    }
}

fn entities_to_svg(entities: &[DrawEntity]) -> String {
    // Calculate bounding box
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    if entities.is_empty() {
        return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"300\"><rect fill=\"#1a1a2e\" width=\"100%\" height=\"100%\"/></svg>");
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
            _ => {}
        }
    }

    let padding = 20.0;
    min_x -= padding;
    min_y -= padding;
    max_x += padding;
    max_y += padding;
    let width = max_x - min_x;
    let height = max_y - min_y;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="100%" height="100%" preserveAspectRatio="xMidYMid meet" style="background:#1a1a2e">"#,
        min_x, -max_y, width, height
    );

    for entity in entities {
        match entity {
            DrawEntity::Line { x1, y1, x2, y2, .. } => {
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#00ff88\" stroke-width=\"1\"/>",
                    x1, -y1, x2, -y2
                ));
            }
            DrawEntity::Circle { cx, cy, radius, .. } => {
                svg.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"#00aaff\" fill=\"none\" stroke-width=\"1\"/>",
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
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" fill=\"#fff\" font-family=\"monospace\" font-size=\"{}\">{}</text>",
                    x, -y, height * 1.5, escaped
                ));
            }
            _ => {}
        }
    }

    svg.push_str("</svg>");
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

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

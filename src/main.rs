// rust-autolisp - AutoLISP Interpreter for AutoCAD 9/10 (DOS era)
//
// A simulation of the 1991 AutoLISP environment used by German electrical
// engineering firms to batch-generate Schaltpläne (circuit diagrams) from
// CSV component lists.
//
// Historical Context:
// In the early 1990s, CAD technicians at firms like "Schmidt & Weber GmbH"
// would load AutoCAD on their 386/486 PCs running MS-DOS, then execute
// LISP programs that read CSV files (exported from dBase or Excel) and
// automatically generated hundreds of DWG files - one for each electrical
// component: Schütz (contactor), Schalter (switch), Kreuzschalter (4-way),
// Sicherung (fuse), and so on.
//
// This Rust interpreter recreates that workflow, allowing you to:
// 1. Run authentic 1991-style .LSP files
// 2. Process the same CSV formats (semicolon-delimited, German encoding)
// 3. Generate simulated DWG output (as JSON or SVG)
// 4. Test without needing DOSBox + AutoCAD

mod interpreter;
mod lexer;
mod parser;

use interpreter::{DrawEntity, Interpreter};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};

const VERSION: &str = "1.0";
const BANNER: &str = r#"
╔═══════════════════════════════════════════════════════════════════╗
║                     RUST-AUTOLISP v1.0                            ║
║         AutoCAD 9/10 AutoLISP Interpreter (DOS Era)               ║
║                                                                   ║
║   "Elektrotechnik Planungsbuero Schmidt & Weber GmbH"             ║
║                    München, 1991                                  ║
║                                                                   ║
║   Recreating the batch Schaltplan generation workflow             ║
╚═══════════════════════════════════════════════════════════════════╝
"#;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => print_usage(&args[0]),
        "-v" | "--version" => println!("rust-autolisp {}", VERSION),
        "-i" | "--interactive" => run_repl(),
        "-r" | "--run" => {
            if args.len() < 3 {
                eprintln!("Error: Missing .lsp file argument");
                return;
            }
            run_file(&args[2], args.get(3).map(|s| s.as_str()));
        }
        "-b" | "--batch" => {
            if args.len() < 4 {
                eprintln!("Error: Usage: {} -b <lisp-file> <csv-file>", args[0]);
                return;
            }
            run_batch(&args[2], &args[3]);
        }
        file if file.ends_with(".lsp") || file.ends_with(".LSP") => {
            run_file(file, args.get(2).map(|s| s.as_str()));
        }
        _ => {
            eprintln!("Unknown option: {}", args[1]);
            print_usage(&args[0]);
        }
    }
}

fn print_usage(prog: &str) {
    println!("{}", BANNER);
    println!("Usage:");
    println!("  {} <file.lsp>              Run a LISP file", prog);
    println!(
        "  {} -r <file.lsp> [output]  Run file, optionally save output",
        prog
    );
    println!(
        "  {} -b <file.lsp> <csv>     Batch mode: process CSV with LISP",
        prog
    );
    println!("  {} -i                      Interactive REPL mode", prog);
    println!("  {} -h                      Show this help", prog);
    println!("  {} -v                      Show version", prog);
    println!();
    println!("Example (simulating 1991 workflow):");
    println!("  {} lisp/CSVREAD.LSP        Load CSV reader", prog);
    println!("  {} lisp/SCHALT.LSP         Generate Schaltpläne", prog);
    println!();
    println!("The interpreter loads .LSP files compatible with AutoCAD Release 10.");
    println!("Output is simulated as JSON (drawing entities) instead of real DWG.");
}

fn run_repl() {
    println!("{}", BANNER);
    println!("AutoLISP REPL - Type expressions, or (quit) to exit");
    println!("Try: (+ 1 2 3)  or  (defun square (x) (* x x))");
    println!();

    let mut interp = Interpreter::new();
    let stdin = io::stdin();

    loop {
        print!("_$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Handle multi-line input (count parens)
        let mut input = line.to_string();
        while !balanced_parens(&input) {
            print!("   ");
            io::stdout().flush().unwrap();
            let mut cont = String::new();
            if stdin.lock().read_line(&mut cont).is_err() {
                break;
            }
            input.push(' ');
            input.push_str(cont.trim());
        }

        // Check for quit
        if input.to_lowercase().contains("(quit)") || input.to_lowercase().contains("(exit)") {
            println!("Auf Wiedersehen!");
            break;
        }

        // Evaluate
        let results = interp.run(&input);

        // Print output buffer
        for msg in &interp.output {
            if msg.is_empty() {
                println!();
            } else {
                print!("{}", msg);
            }
        }
        interp.output.clear();

        // Print results
        for result in results {
            println!("{}", result);
        }
    }
}

fn balanced_parens(s: &str) -> bool {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut prev_char = ' ';

    for c in s.chars() {
        if c == '"' && prev_char != '\\' {
            in_string = !in_string;
        } else if !in_string {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        prev_char = c;
    }

    depth == 0
}

fn run_file(path: &str, output_path: Option<&str>) {
    println!("{}", BANNER);
    println!("Loading: {}", path);
    println!();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path, e);
            return;
        }
    };

    let mut interp = Interpreter::new();

    // If the file references other files (like CSVREAD.LSP), try to load them
    // from the same directory
    let base_dir = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    // Pre-load common dependencies if they exist
    for dep in &["CSVREAD.LSP", "EBLOCKS.LSP"] {
        let dep_path = base_dir.join(dep);
        if dep_path.exists() {
            if let Ok(dep_content) = fs::read_to_string(&dep_path) {
                println!("Auto-loading dependency: {}", dep);
                interp.run(&dep_content);
            }
        }
    }

    // Run the main file
    let results = interp.run(&content);

    // Print output
    for msg in &interp.output {
        if msg.is_empty() {
            println!();
        } else {
            print!("{}", msg);
        }
    }
    println!();

    // Print results
    println!("--- Results ---");
    for result in &results {
        println!("{}", result);
    }

    // Print command log
    if !interp.command_log.is_empty() {
        println!();
        println!("--- AutoCAD Commands Issued ---");
        for cmd in &interp.command_log {
            println!("Command: {}", cmd);
        }
    }

    // Print drawing entities
    if !interp.drawing.entities.is_empty() {
        println!();
        println!(
            "--- Drawing Entities Created: {} ---",
            interp.drawing.entities.len()
        );

        // Show first 10 only
        for (i, entity) in interp.drawing.entities.iter().take(10).enumerate() {
            println!("{}: {:?}", i + 1, entity);
        }
        if interp.drawing.entities.len() > 10 {
            println!(
                "... and {} more entities",
                interp.drawing.entities.len() - 10
            );
        }

        // Auto-generate output files
        let base_name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let output_dir = std::path::Path::new("output");
        if !output_dir.exists() {
            let _ = fs::create_dir(output_dir);
        }

        // Save as JSON
        let json_path = output_dir.join(format!("{}.json", base_name));
        save_entities_json(&interp.drawing.entities, json_path.to_str().unwrap());

        // Save as SVG
        let svg_path = output_dir.join(format!("{}.svg", base_name));
        save_entities_svg(&interp.drawing.entities, svg_path.to_str().unwrap());

        // If custom output path specified, save there too
        if let Some(out_path) = output_path {
            if out_path.ends_with(".svg") {
                save_entities_svg(&interp.drawing.entities, out_path);
            } else {
                save_entities_json(&interp.drawing.entities, out_path);
            }
        }
    }
}

fn run_batch(lisp_path: &str, csv_path: &str) {
    println!("{}", BANNER);
    println!("=== BATCH PROCESSING MODE ===");
    println!("LISP Program: {}", lisp_path);
    println!("CSV Data:     {}", csv_path);
    println!();

    // Load LISP code
    let lisp_content = match fs::read_to_string(lisp_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading LISP file: {}", e);
            return;
        }
    };

    // Check CSV exists
    if !std::path::Path::new(csv_path).exists() {
        eprintln!("Error: CSV file not found: {}", csv_path);
        return;
    }

    let mut interp = Interpreter::new();

    // Load dependencies
    let base_dir = std::path::Path::new(lisp_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    for dep in &["CSVREAD.LSP", "EBLOCKS.LSP"] {
        let dep_path = base_dir.join(dep);
        if dep_path.exists() {
            if let Ok(dep_content) = fs::read_to_string(&dep_path) {
                println!("Loading: {}", dep);
                interp.run(&dep_content);
            }
        }
    }

    // Load main program
    println!("Loading: {}", lisp_path);
    interp.run(&lisp_content);

    // Load CSV data
    println!("\nProcessing CSV: {}", csv_path);
    let csv_load_cmd = format!(r#"(setq *batch-data* (CSV-LOADH "{}"))"#, csv_path);
    interp.run(&csv_load_cmd);

    // Check if data was loaded
    let check = interp.run("(length *batch-data*)");
    if let Some(parser::Expr::Integer(count)) = check.first() {
        println!("Loaded {} records from CSV\n", count);

        // Process each record
        for i in 0..*count {
            let process_cmd = format!(
                r#"(progn
                     (setq *current-row* (nth {} *batch-data*))
                     (princ (strcat "\n[" (itoa {}) "] "))
                     (princ (CSV-GET "BEZEICHNUNG" *current-row*))
                     (SCHALT-PROCESS *current-row* {})
                   )"#,
                i,
                i + 1,
                i
            );
            interp.run(&process_cmd);
        }
    }

    // Print all output
    println!();
    for msg in &interp.output {
        if msg.is_empty() {
            println!();
        } else {
            print!("{}", msg);
        }
    }
    println!();

    // Generate output files (simulated)
    let output_dir = std::path::Path::new("output");
    if !output_dir.exists() {
        let _ = fs::create_dir(output_dir);
    }

    println!("\n=== Generated Files (Simulated DWG as JSON) ===");
    if !interp.drawing.entities.is_empty() {
        let json_path = output_dir.join("BATCH_OUTPUT.json");
        save_entities_json(&interp.drawing.entities, json_path.to_str().unwrap());
        println!("Entities saved to: {}", json_path.display());
    }

    // Summary
    println!("\n=== Batch Processing Complete ===");
    println!("Commands issued: {}", interp.command_log.len());
    println!("Entities created: {}", interp.drawing.entities.len());
}

fn save_entities_json(entities: &[DrawEntity], path: &str) {
    let mut json = String::from("[\n");

    for (i, entity) in entities.iter().enumerate() {
        if i > 0 {
            json.push_str(",\n");
        }
        json.push_str("  ");
        json.push_str(&entity_to_json(entity));
    }

    json.push_str("\n]");

    if let Err(e) = fs::write(path, &json) {
        eprintln!("Error writing JSON: {}", e);
    } else {
        println!("Saved: {}", path);
    }
}

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
                r#"{{"type": "LINE", "x1": {}, "y1": {}, "x2": {}, "y2": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "CIRCLE", "cx": {}, "cy": {}, "radius": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "ARC", "cx": {}, "cy": {}, "radius": {}, "start": {}, "end": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "TEXT", "x": {}, "y": {}, "height": {}, "text": "{}", "layer": "{}"}}"#,
                x, y, height, escaped, layer
            )
        }
        DrawEntity::Point { x, y, layer } => {
            format!(
                r#"{{"type": "POINT", "x": {}, "y": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "INSERT", "block": "{}", "x": {}, "y": {}, "scale": {}, "rotation": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "PIN", "name": "{}", "number": "{}", "type": "{}", "style": "{}", "x": {}, "y": {}, "length": {}, "rotation": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "PROPERTY", "key": "{}", "value": "{}", "x": {}, "y": {}, "rotation": {}, "height": {}, "visible": {}, "layer": "{}"}}"#,
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
                r#"{{"type": "PAD", "name": "{}", "type": "{}", "shape": "{}", "x": {}, "y": {}, "width": {}, "height": {}, "drill": {}, "rotation": {}, "layers": "{}"}}"#,
                name, ptype, shape, x, y, width, height, drill, rotation, layers
            )
        }
    }
}

fn save_entities_svg(entities: &[DrawEntity], path: &str) {
    // Calculate bounding box
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

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
            DrawEntity::Text { x, y, .. } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x + 100.0);
                max_y = max_y.max(*y + 20.0);
            }
            DrawEntity::Insert { x, y, .. } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x + 50.0);
                max_y = max_y.max(*y + 50.0);
            }
            DrawEntity::Pin { x, y, length, .. } => {
                min_x = min_x.min(*x - length);
                min_y = min_y.min(*y - length);
                max_x = max_x.max(*x + length);
                max_y = max_y.max(*y + length);
            }
            DrawEntity::Property { .. } => {
                // Ignore
            }
            _ => {}
        }
    }

    // Add padding
    let padding = 50.0;
    min_x -= padding;
    min_y -= padding;
    max_x += padding;
    max_y += padding;

    let width = max_x - min_x;
    let height = max_y - min_y;

    // Scale to fit in 1200px width
    let scale = if width > 0.0 { 1200.0 / width } else { 1.0 };
    let svg_width = width * scale;
    let svg_height = height * scale;

    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="{} {} {} {}"
     width="{}" height="{}"
     style="background-color: #1a1a2e;">
  <title>AutoLISP Drawing - rust-autolisp</title>
  <desc>Generated by rust-autolisp - AutoCAD 9/10 Simulator</desc>
  <defs>
    <style>
      .line {{ stroke: #00ff88; stroke-width: 1; fill: none; }}
      .circle {{ stroke: #00aaff; stroke-width: 1; fill: none; }}
      .text {{ fill: #ffffff; font-family: monospace; }}
      .grid {{ stroke: #333355; stroke-width: 0.5; }}
    </style>
  </defs>

  <!-- Grid -->
  <g class="grid">
"#,
        min_x, -max_y, width, height, svg_width, svg_height
    );

    // Draw grid
    let grid_step = 100.0;
    let mut gx = (min_x / grid_step).floor() * grid_step;
    while gx <= max_x {
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" />\n",
            gx, -max_y, gx, -min_y
        ));
        gx += grid_step;
    }
    let mut gy = (min_y / grid_step).floor() * grid_step;
    while gy <= max_y {
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" />\n",
            min_x, -gy, max_x, -gy
        ));
        gy += grid_step;
    }
    svg.push_str("  </g>\n\n  <!-- Entities -->\n  <g>\n");

    // Draw entities
    for entity in entities {
        match entity {
            DrawEntity::Line { x1, y1, x2, y2, .. } => {
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
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"{}\">{}</text>\n",
                    x,
                    -y,
                    height * 1.5,
                    escaped
                ));
            }
            DrawEntity::Insert {
                block_name, x, y, ..
            } => {
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"40\" height=\"40\" stroke=\"#ff8800\" fill=\"none\" />\n",
                    x, -y - 40.0
                ));
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" class=\"text\" font-size=\"8\">{}</text>\n",
                    x + 2.0,
                    -y - 15.0,
                    block_name
                ));
            }
            _ => {}
        }
    }

    svg.push_str("  </g>\n\n");

    // Add title
    svg.push_str("  <!-- Title -->\n");
    svg.push_str(&format!(
        "  <text x=\"{}\" y=\"{}\" fill=\"#00ff88\" font-family=\"monospace\" font-size=\"16\">\n",
        min_x + 10.0,
        -max_y + 20.0
    ));
    svg.push_str("    rust-autolisp - AutoCAD 9/10 Simulator\n");
    svg.push_str("  </text>\n");
    svg.push_str(&format!(
        "  <text x=\"{}\" y=\"{}\" fill=\"#888888\" font-family=\"monospace\" font-size=\"10\">\n",
        min_x + 10.0,
        -max_y + 35.0
    ));
    svg.push_str(&format!(
        "    Entities: {} | Schmidt &amp; Weber GmbH 1991\n",
        entities.len()
    ));
    svg.push_str("  </text>\n");

    svg.push_str("</svg>\n");

    if let Err(e) = fs::write(path, &svg) {
        eprintln!("Error writing SVG: {}", e);
    } else {
        println!("Saved SVG: {}", path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_evaluation() {
        let mut interp = Interpreter::new();
        let results = interp.run("(+ 1 2 3)");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_defun_and_call() {
        let mut interp = Interpreter::new();
        interp.run("(defun double (x) (* x 2))");
        let results = interp.run("(double 21)");
        if let Some(parser::Expr::Integer(n)) = results.first() {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected integer 42");
        }
    }

    #[test]
    fn test_string_operations() {
        let mut interp = Interpreter::new();
        let results = interp.run(r#"(strcat "Schütz" "-" "K1")"#);
        if let Some(parser::Expr::String(s)) = results.first() {
            assert_eq!(s, "Schütz-K1");
        }
    }
}

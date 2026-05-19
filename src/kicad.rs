// KiCad Export Module
// Handles exporting DrawEntities to KiCad S-expression format

use crate::interpreter::DrawEntity;

/// Exports a list of DrawEntities to a KiCad Symbol Library (.kicad_sym) format
pub fn to_kicad_sym(entities: &[DrawEntity], library_name: &str, symbol_name: &str) -> String {
    let mut sexpr = String::new();

    // Standard header for V6+ symbol library
    sexpr.push_str("(kicad_symbol_lib (version 20211014) (generator acadlisp)\n");

    // Start symbol definition
    sexpr.push_str(&format!(
        "  (symbol \"{}:{}\" (in_bom yes) (on_board yes)\n",
        library_name, symbol_name
    ));

    // Separate properties, pins, and graphics
    let mut properties = Vec::new();
    let mut pins = Vec::new();
    let mut graphics = Vec::new();

    for entity in entities {
        match entity {
            DrawEntity::Property { .. } => properties.push(entity),
            DrawEntity::Pin { .. } => pins.push(entity),
            _ => graphics.push(entity),
        }
    }

    // Write properties
    // Ensure mandatory properties exist if not provided: Reference, Value, Footprint, Datasheet
    let mut has_ref = false;
    let mut has_val = false;
    let mut has_fp = false;
    let mut has_ds = false;

    for (i, prop) in properties.iter().enumerate() {
        if let DrawEntity::Property {
            key,
            value,
            x,
            y,
            rotation,
            height,
            visible,
            ..
        } = prop
        {
            if key == "Reference" {
                has_ref = true;
            }
            if key == "Value" {
                has_val = true;
            }
            if key == "Footprint" {
                has_fp = true;
            }
            if key == "Datasheet" {
                has_ds = true;
            }

            sexpr.push_str(&format!(
                "    (property \"{}\" \"{}\" (id {}) (at {} {} {}) (effects (font (size {} {})) {}))\n",
                key,
                escape_string(value),
                i,
                x, y, rotation,
                height, height,
                if *visible { "" } else { "hide" }
            ));
        }
    }

    // Add default properties if missing
    let next_id = properties.len();
    if !has_ref {
        sexpr.push_str(&format!("    (property \"Reference\" \"U1\" (id {}) (at 0 5 0) (effects (font (size 1.27 1.27))))\n", next_id));
    }
    if !has_val {
        sexpr.push_str(&format!("    (property \"Value\" \"{}\" (id {}) (at 0 -5 0) (effects (font (size 1.27 1.27))))\n", symbol_name, next_id + 1));
    }
    if !has_fp {
        sexpr.push_str(&format!("    (property \"Footprint\" \"\" (id {}) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))\n", next_id + 2));
    }
    if !has_ds {
        sexpr.push_str(&format!("    (property \"Datasheet\" \"\" (id {}) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))\n", next_id + 3));
    }

    // Symbol body (graphics and pins)
    sexpr.push_str(&format!("    (symbol \"{}:1_1\"\n", symbol_name));

    // Graphics
    for entity in graphics {
        match entity {
            DrawEntity::Line { x1, y1, x2, y2, .. } => {
                sexpr.push_str(&format!(
                    "      (polyline (pts (xy {} {}) (xy {} {})) (stroke (width 0) (type default) (color 0 0 0 0)) (fill (type none)))\n",
                    x1, y1, x2, y2
                ));
            }
            DrawEntity::Circle { cx, cy, radius, .. } => {
                sexpr.push_str(&format!(
                    "      (circle (center {} {}) (radius {}) (stroke (width 0) (type default) (color 0 0 0 0)) (fill (type none)))\n",
                    cx, cy, radius
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
                let mid_angle = (start_angle + end_angle) / 2.0;
                let mid_x = cx + radius * mid_angle.cos();
                let mid_y = cy + radius * mid_angle.sin();

                sexpr.push_str(&format!(
                    "      (arc (start {} {}) (mid {} {}) (end {} {}) (stroke (width 0) (type default) (color 0 0 0 0)) (fill (type none)))\n",
                    start_x, start_y, mid_x, mid_y, end_x, end_y
                ));
            }
            DrawEntity::Text {
                x, y, height, text, ..
            } => {
                sexpr.push_str(&format!(
                    "      (text \"{}\" (at {} {} 0) (effects (font (size {} {}))))\n",
                    escape_string(text),
                    x,
                    y,
                    height,
                    height
                ));
            }
            DrawEntity::Insert {
                block_name: _,
                x: _,
                y: _,
                ..
            } => {
                // Ignored
            }
            _ => {}
        }
    }

    // Pins
    for pin in pins {
        if let DrawEntity::Pin {
            name,
            number,
            etype,
            style,
            x,
            y,
            length,
            rotation,
            ..
        } = pin
        {
            sexpr.push_str(&format!(
                "      (pin {} {} (at {} {} {}) (length {}) \n",
                etype, style, x, y, rotation, length
            ));
            sexpr.push_str(&format!(
                "        (name \"{}\" (effects (font (size 1.27 1.27))))\n",
                escape_string(name)
            ));
            sexpr.push_str(&format!(
                "        (number \"{}\" (effects (font (size 1.27 1.27))))\n",
                escape_string(number)
            ));
            sexpr.push_str("      )\n");
        }
    }

    sexpr.push_str("    )\n");
    sexpr.push_str("  )\n");
    sexpr.push_str(")\n");

    sexpr
}

/// Exports a list of DrawEntities to a KiCad Footprint (.kicad_mod) format
pub fn to_kicad_mod(entities: &[DrawEntity], footprint_name: &str) -> String {
    let mut sexpr = String::new();

    // Header
    sexpr.push_str(&format!(
        "(footprint \"{}\" (layer \"F.Cu\")\n",
        footprint_name
    ));
    sexpr.push_str("  (tedit 0)\n"); // Timestamp edit 0

    // Properties (Reference, Value) need to be handled if present, or defaults added.
    // In footprint files, these are fp_text objects.
    // Typically Reference is on F.SilkS and Value on F.Fab.

    let mut has_ref = false;
    let mut has_val = false;

    // Process entities
    for entity in entities {
        match entity {
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
                // (pad "1" smd rect (at 0 0) (size 1.5 1.5) (layers "F.Cu" "F.Paste" "F.Mask"))
                // layers string should be space separated quoted strings, but we store it as a single string?
                // Let's assume the user passes "F.Cu" or "F.Cu F.Mask".
                // We need to format it properly. If it's just a raw string like "F.Cu", we can quote it.
                // If it's a list like "F.Cu,F.Mask", we should split and quote.

                let layer_str = layers
                    .split(',')
                    .map(|s| format!("\"{}\"", s.trim()))
                    .collect::<Vec<_>>()
                    .join(" ");

                sexpr.push_str(&format!(
                    "  (pad \"{}\" {} {} (at {} {} {}) (size {} {}) (drill {}) (layers {}))\n",
                    name, ptype, shape, x, y, rotation, width, height, drill, layer_str
                ));
            }
            DrawEntity::Line {
                x1,
                y1,
                x2,
                y2,
                layer,
            } => {
                // (fp_line (start x y) (end x y) (layer "Layer") (width 0.15))
                sexpr.push_str(&format!(
                    "  (fp_line (start {} {}) (end {} {}) (layer \"{}\") (stroke (width 0.15) (type solid)))\n",
                    x1, y1, x2, y2, layer
                ));
            }
            DrawEntity::Circle {
                cx,
                cy,
                radius,
                layer,
            } => {
                // (fp_circle (center x y) (end x y) (layer "Layer") ...)
                // End point is center + radius
                let end_x = cx + radius;
                sexpr.push_str(&format!(
                    "  (fp_circle (center {} {}) (end {} {}) (layer \"{}\") (stroke (width 0.15) (type solid)))\n",
                    cx, cy, end_x, cy, layer
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
                // (fp_arc (start x y) (mid x y) (end x y) ...)
                let start_x = cx + radius * start_angle.cos();
                let start_y = cy + radius * start_angle.sin();
                let end_x = cx + radius * end_angle.cos();
                let end_y = cy + radius * end_angle.sin();
                let mid_angle = (start_angle + end_angle) / 2.0;
                let mid_x = cx + radius * mid_angle.cos();
                let mid_y = cy + radius * mid_angle.sin();

                sexpr.push_str(&format!(
                    "  (fp_arc (start {} {}) (mid {} {}) (end {} {}) (layer \"{}\") (stroke (width 0.15) (type solid)))\n",
                    start_x, start_y, mid_x, mid_y, end_x, end_y, layer
                ));
            }
            DrawEntity::Text {
                x,
                y,
                height,
                text,
                layer,
            } => {
                // (fp_text user "Text" (at x y rot) (layer "Layer") (effects (font (size h h) (thickness t))))
                // Check if it's special text
                let type_str = if text.starts_with("REF") {
                    "reference"
                } else if text.starts_with("VAL") {
                    "value"
                } else {
                    "user"
                };

                if type_str == "reference" {
                    has_ref = true;
                }
                if type_str == "value" {
                    has_val = true;
                }

                sexpr.push_str(&format!(
                    "  (fp_text {} \"{}\" (at {} {} 0) (layer \"{}\") (effects (font (size {} {}) (thickness 0.15))))\n",
                    type_str, escape_string(text), x, y, layer, height, height
                ));
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
                // Properties in footprints are often just text or specific fields.
                // We'll map them to fp_text if possible, or ignore if they don't fit the model.
                // Standard properties Reference/Value map to text.
                let type_str = if key == "Reference" {
                    "reference"
                } else if key == "Value" {
                    "value"
                } else {
                    "user"
                };

                if type_str == "reference" {
                    has_ref = true;
                }
                if type_str == "value" {
                    has_val = true;
                }

                sexpr.push_str(&format!(
                    "  (fp_text {} \"{}\" (at {} {} {}) (layer \"{}\") (effects (font (size {} {}) (thickness 0.15)) {}))\n",
                    type_str, escape_string(value), x, y, rotation, layer, height, height,
                    if *visible { "" } else { "hide" }
                ));
            }
            _ => {}
        }
    }

    // Add default Ref/Val if missing
    if !has_ref {
        sexpr.push_str("  (fp_text reference \"REF**\" (at 0 -0.5) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.15))))\n");
    }
    if !has_val {
        sexpr.push_str(&format!("  (fp_text value \"{}\" (at 0 1) (layer \"F.Fab\") (effects (font (size 1 1) (thickness 0.15))))\n", footprint_name));
    }

    sexpr.push_str(")\n");
    sexpr
}

fn escape_string(s: &str) -> String {
    s.replace('"', "\\\"").replace('\\', "\\\\")
}

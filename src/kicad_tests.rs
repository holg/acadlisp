#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::kicad;

    #[test]
    fn test_kicad_symbol_generation() {
        let mut interp = Interpreter::new();

        let code = r#"
            (kicad-prop "Reference" "U1" 0 5 0 1.27 1)
            (kicad-prop "Value" "TEST_SYM" 0 -5 0 1.27 1)
            (command "LINE" '(-5 5) '(5 5) "")
            (kicad-pin "IN" "1" "input" "line" -7.54 0 2.54 0)
        "#;

        interp.run(code);
        let sym = kicad::to_kicad_sym(&interp.drawing.entities, "MyLib", "MySym");

        assert!(sym.contains("(kicad_symbol_lib"));
        assert!(sym.contains("(symbol \"MyLib:MySym\""));
        assert!(sym.contains("(pin input line"));
    }

    #[test]
    fn test_kicad_footprint_generation() {
        let mut interp = Interpreter::new();

        let code = r#"
            (command "LAYER" "S" "F.SilkS" "")
            (command "LINE" '(-2 2) '(2 2) "")
            (command "LINE" '(2 2) '(2 -2) "")

            (kicad-pad "1" "smd" "rect" -1.5 0 1.0 1.5 0 0 "F.Cu")
            (kicad-pad "2" "smd" "rect" 1.5 0 1.0 1.5 0 0 "F.Cu")
        "#;

        interp.run(code);
        let fp = kicad::to_kicad_mod(&interp.drawing.entities, "TEST_FP");

        println!("{}", fp);

        assert!(fp.contains("(footprint \"TEST_FP\""));
        assert!(fp.contains("(fp_line (start -2 2) (end 2 2) (layer \"F.SilkS\")"));
        assert!(fp.contains(
            "(pad \"1\" smd rect (at -1.5 0 0) (size 1 1.5) (drill 0) (layers \"F.Cu\"))"
        ));
    }
}

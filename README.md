# acadlisp

AutoLISP interpreter and mini CAD engine in Rust, targeting WebAssembly.

Emulates CSV/TPL/LSP workflows from AutoCAD 9/10 (DOS era) for educational and archival purposes.

## Features

- AutoLISP interpreter with core functions
- CAD primitives: LINE, CIRCLE, ARC, TEXT, INSERT (blocks)
- CSV data reading for parametric drawings
- DXF export
- **KiCad Export** (Symbols .kicad_sym and Footprints .kicad_mod)
- Runs natively and in browser (WASM)

## Try it

Live demo: [acadlisp.de](https://acadlisp.de)

## Usage

### As a library

```rust
use acadlisp::Engine;

let mut engine = Engine::new();
engine.eval("(setq x 10)");
engine.eval("(command \"LINE\" '(0 0) '(100 100) \"\")");

let entities = engine.get_entities();
let dxf = engine.export_dxf();
```

### KiCad Export

You can use AutoLISP to define parametric KiCad symbols and footprints.

#### Generating a Symbol (.kicad_sym)

```lisp
; Define properties
(kicad-prop "Reference" "U1" 0 5 0 1.27 1)
(kicad-prop "Value" "NE555" 0 -5 0 1.27 1)

; Draw symbol graphics
(command "LINE" '(-5 5) '(5 5) "")
(command "LINE" '(5 5) '(5 -5) "")
(command "LINE" '(5 -5) '(-5 -5) "")
(command "LINE" '(-5 -5) '(-5 5) "")

; Add pins
; (kicad-pin name number type style x y length rotation)
(kicad-pin "GND" "1" "power_in" "line" 0 -7.54 2.54 90)
(kicad-pin "TRIG" "2" "input" "line" -7.54 2.54 2.54 0)
(kicad-pin "OUT" "3" "output" "line" 7.54 0 2.54 180)
```

Export via Rust/WASM API: `engine.get_kicad_sym("MyLibrary", "MySymbol")`

#### Generating a Footprint (.kicad_mod)

```lisp
; Switch to Silk Screen layer for outline
(command "LAYER" "S" "F.SilkS" "")
(command "LINE" '(-2 2) '(2 2) "")
(command "LINE" '(2 2) '(2 -2) "")
(command "LINE" '(2 -2) '(-2 -2) "")
(command "LINE" '(-2 -2) '(-2 2) "")

; Add pads
; (kicad-pad name type shape x y width height drill rotation layers)
(kicad-pad "1" "smd" "rect" -1.5 0 1.0 1.5 0 0 "F.Cu")
(kicad-pad "2" "smd" "rect" 1.5 0 1.0 1.5 0 0 "F.Cu")
```

Export via Rust/WASM API: `engine.get_kicad_mod("MyFootprint")`

### CLI

```bash
cargo run --bin acadlisp-cli -- samples/TEST.LSP
```

### WASM

```bash
trunk serve
# Open http://localhost:8080
```

## Supported AutoLISP Functions

### Core
`setq` `defun` `if` `cond` `while` `repeat` `progn` `lambda` `apply` `mapcar` `foreach`

### Math
`+` `-` `*` `/` `1+` `1-` `abs` `sin` `cos` `atan` `sqrt` `expt` `min` `max` `rem` `gcd`

### Comparison
`=` `/=` `<` `>` `<=` `>=` `eq` `equal`

### Logic
`and` `or` `not` `null`

### List
`car` `cdr` `cons` `list` `append` `reverse` `length` `nth` `member` `assoc` `subst`

### String
`strcat` `strlen` `substr` `strcase` `atoi` `atof` `itoa` `rtos` `read`

### I/O
`print` `princ` `prin1` `terpri` `prompt`

### CAD
`command` `getvar` `setvar` `entget` `entmake` `entmod` `ssget` `sslength` `ssname`

### KiCad
`kicad-pin` `kicad-prop` `kicad-pad`

## Building

```bash
# Native
cargo build --release

# WASM
trunk build --release
```

## License

MIT

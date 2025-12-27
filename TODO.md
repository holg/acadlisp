# KiCad Export Support

The goal is to extend `acadlisp` to serve as a parametric generator for KiCad artifacts (symbols, footprints, and 3D models). This leverages the engine's Lisp interpreter to script complex geometries and export them directly to KiCad's S-expression based file formats.

## 1. Research & Design
- [x] **Analyze KiCad File Formats:** Deep dive into format specifications for `.kicad_sym` (symbols) and `.kicad_mod` (footprints).
- [x] **Map Primitives:** Determine how `acadlisp` primitives (`LINE`, `CIRCLE`, `ARC`, `TEXT`) map to KiCad's graphical elements.
- [x] **Define Lisp Extensions:** Design new Lisp functions needed for specific KiCad entities not covered by generic CAD commands (e.g., pins, pads, electrical properties).

## 2. Symbol Export (`.kicad_sym`)
- [x] **Implement `kicad-pin` Function:** Add a Lisp command to define symbol pins (number, name, electrical type, position).
- [x] **Implement `kicad-property` Function:** Support adding metadata fields (Reference, Value, Footprint, Datasheet).
- [x] **Create Symbol Exporter:** Implement a Rust module to traverse the `DrawEntity` list and serialize it into the `.kicad_sym` S-expression format.
- [x] **Verify Symbol Geometry:** Ensure lines, circles, and arcs are correctly transformed and scaled for schematic grids.

## 3. Footprint Export (`.kicad_mod`)
- [x] **Verify Feasibility:** Confirm `acadlisp`'s geometric engine can handle footprint-specific needs (layers like F.Cu, F.SilkS, accurate dimensions).
- [x] **Implement `kicad-pad` Function:** Add a Lisp command to define pads (SMD/Through-hole, size, shape, layer stack).
- [x] **Create Footprint Exporter:** Implement a Rust module to export entities to `.kicad_mod`.
- [x] **Layer Management:** Add support for mapping drawing layers (e.g., "0", "SILK") to KiCad technical layers.

## 4. 3D Model Export (Future Scope)
- [ ] **Explore 3D Capabilities:** Investigate if `acadlisp` needs extension to support Z-axis or 3D primitives, or if it can generate OpenSCAD/STEP scripts.
- [ ] **Define 3D Scripting:** Design Lisp commands for basic 3D shapes (extrusion, union, difference) if native generation is pursued.

## 5. Verification & Examples
- [x] **Create Test Suite:** Develop a set of standard Lisp scripts that generate known KiCad components (e.g., "555 Timer Symbol", "SOIC-8 Footprint").
- [ ] **Integration Test:** verifying that the exported files can be opened in KiCad without errors.

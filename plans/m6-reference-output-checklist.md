# M6 Reference Output Checklist

This checklist validates that the Rust export stack produces outputs that load correctly in downstream tools.

## Prerequisites

- Build the reference outputs:
  - `cargo run -p ergogen-export --example reference_outputs -- fixtures/upstream/fixtures/big.yaml`
- Outputs land at:
  - `target/reference-outputs/big/`
  - `target/reference-outputs/big/outlines/`
  - `target/reference-outputs/big/cases/`
  - `target/reference-outputs/big/pcbs/`

Files of interest:
- `target/reference-outputs/big/outlines/export.dxf`
- `target/reference-outputs/big/outlines/export.svg`
- `target/reference-outputs/big/cases/export.jscad`
- `target/reference-outputs/big/pcbs/export.kicad_pcb`

## DXF (2D) — QCAD / LibreCAD / Inkscape

1. Open `export.dxf`.
2. Verify a single rectangular outline is visible.
3. Confirm dimensions match 18×18 (units match expected mm scale).
4. Zoom to extents and ensure no stray entities.
5. (Optional) Measure corner points for expected coordinates.

Pass if: outline renders cleanly, bounding box is 18×18, no extra geometry.

## SVG (2D) — Inkscape / Browser

1. Open `export.svg` in a browser or Inkscape.
2. Verify a single rectangular outline is visible.
3. Check that the outline matches the DXF (size and orientation).
4. Ensure stroke appears and there are no fills.

Pass if: shape matches the DXF and renders without artifacts.

## JSCAD (3D) — JSCAD Desktop or Web

1. Open `export.jscad` in JSCAD (this output now targets JSCAD v2+ via `@jscad/modeling`).
2. Run/compile the script.
3. Confirm a simple extruded rectangle is visible.
4. Inspect dimensions (roughly 18×18 footprint, 1 unit extrusion).

Pass if: model renders without errors and matches expected size.

## KiCad PCB — KiCad 5/6/7/8

1. Open `export.kicad_pcb` in KiCad.
2. Confirm the board loads without warnings.
3. Inspect layers list for standard layers.
4. Ensure no footprint errors or missing nets.

Pass if: file loads cleanly and the board canvas renders (even if empty).

## Record Results

- Date:
- Tool versions (QCAD/Inkscape/JSCAD/KiCad):
- Pass/Fail per section:
  - DXF:
  - SVG:
  - JSCAD:
  - KiCad:
- Notes / screenshots:

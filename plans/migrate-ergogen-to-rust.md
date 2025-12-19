# Migrate Ergogen to Rust

**Type:** Major Rewrite
**Priority:** High
**Status:** Planning
**Created:** December 18, 2025

---

## Overview

Migrate Ergogen from JavaScript/Node.js to Rust, including porting makerjs functionality and other dependencies that lack Rust equivalents. This is a complete rewrite that will result in:

- Native binary CLI with significant performance improvements (3-10x expected)
- WebAssembly build for browser deployment at ergogen.xyz
- Type-safe codebase with compile-time error checking
- Parallelizable geometry operations

---

## Problem Statement / Motivation

**Why migrate to Rust?**

1. **Performance**: Keyboard generation with complex outlines can be slow in JS. Rust offers 3-10x speedup potential with SIMD and parallelization
2. **Type Safety**: Current JS codebase relies on runtime validation (assert.js). Rust catches type errors at compile time
3. **Memory Efficiency**: No garbage collection pauses, better memory footprint for large designs
4. **WebAssembly**: Rust compiles to WASM with smaller bundle sizes than JS + dependencies
5. **Maintainability**: Strong type system makes refactoring safer

**Current State:**
- ~2,554 lines of JavaScript
- 100% test coverage with 68 integration fixtures (upstream); this repo has imported 64 YAML fixtures from `test/` and runs them via `crates/ergogen-cli/tests/upstream_suite.rs` (61 reference artifacts with 0 skips for points/outlines/cases/pcbs/footprints) plus the upstream CLI fixtures via `crates/ergogen-cli/tests/cli_suite.rs` (logs/errors + 73 reference artifacts). SVG export is additionally covered by unit tests in `crates/ergogen-export/tests/svg.rs`.
- Heavy dependency on makerjs (no Rust equivalent)
- Used by thousands of keyboard designers

---

## Critical Architectural Decisions Required

### Decision 1: JavaScript Execution Strategy

**Question:** How to handle JavaScript config files and custom footprints/templates?

| Option | Pros | Cons | Recommendation |
|--------|------|------|----------------|
| **A. Remove JS support** | Simple, pure Rust | Breaking change, custom footprints broken | Not recommended |
| **B. Embed Boa (JS engine)** | Full compatibility | Large binary, complex, defeats performance goal | Not recommended |
| **C. Declarative footprints + templates** | Clean architecture, portable | Requires footprint conversion, limits flexibility | **Recommended** |
| **D. Rhai/Lua scripting** | Fast, small, embeddable | Learning curve, different syntax | Viable alternative |

**Recommended Approach (Option C):**
- Convert footprints to declarative YAML format with template interpolation (Tera)
- Provide migration tool for simple JS footprints
- Complex footprints require manual conversion or Rhai scripting (Option D)

```yaml
# New declarative footprint format
name: mx_switch
author: ergogen community
description: MX compatible switch footprint
version: 1.0.0

params:
  from: { type: net, required: true }
  to: { type: net, required: true }
  hotswap: { type: boolean, default: false }
  reverse: { type: boolean, default: false }

primitives:
  - type: pad
    at: [0, -5.9]
    size: [2.55, 2.5]
    layers: [F.Cu, F.Paste, F.Mask]
    net: "{{ from }}"

  - type: pad
    at: [0, 5.9]
    size: [2.55, 2.5]
    layers: [F.Cu, F.Paste, F.Mask]
    net: "{{ to }}"
```

### Decision 2: Geometry Library Stack

**Recommended Stack:**

| Component | Library | Purpose |
|-----------|---------|---------|
| Curves & Paths | `kurbo` | Bezier curves, arcs, paths |
| Polygons | `geo` | Points, lines, polygons |
| Boolean Ops | `i_overlay` | Fast polygon boolean operations |
| Complex Ops | `cavalier_contours` | Arc-aware offsetting, filleting |

**Why this combination:**
- `kurbo` handles the curve math that makerjs uses
- `geo` provides spatial primitives and algorithms
- `i_overlay` is faster than `geo-booleanop` for polygon booleans
- `cavalier_contours` preserves arcs during offset operations (critical for outlines)

### Decision 3: Output Format Compatibility

**Requirement:** Semantic equivalence, not byte-identical outputs.

Acceptable differences:
- Floating point precision formatting (14.0 vs 14.000000)
- Element ordering in SVG/DXF
- Whitespace and formatting
- Comments

Must match:
- Coordinate values (within epsilon)
- Topology (same shapes, same connections)
- Net assignments in KiCAD

---

## Technical Approach

### Core Architecture

```
ergogen-rs/
├── crates/
│   ├── ergogen-core/          # Core types and traits
│   │   ├── point.rs           # Point struct
│   │   ├── anchor.rs          # Anchor resolution
│   │   ├── filter.rs          # Point filtering
│   │   └── config.rs          # Config types
│   │
│   ├── ergogen-geometry/      # 2D geometry operations
│   │   ├── primitives.rs      # Rectangle, Circle, Polygon
│   │   ├── path.rs            # Path with segments
│   │   ├── boolean.rs         # Union, Subtract, Intersect
│   │   ├── transform.rs       # Scale, Rotate, Mirror
│   │   └── hull.rs            # Convex/Concave hull
│   │
│   ├── ergogen-parser/        # Config parsing and preprocessing
│   │   ├── yaml.rs            # YAML parsing
│   │   ├── prepare.rs         # Unnest, inherit, parameterize
│   │   ├── units.rs           # Unit variable resolution
│   │   └── expr.rs            # Math expression evaluation
│   │
│   ├── ergogen-layout/        # Point generation
│   │   ├── zones.rs           # Zone-based layout
│   │   ├── columns.rs         # Column handling
│   │   ├── rows.rs            # Row handling
│   │   └── mirror.rs          # Mirror axis calculation
│   │
│   ├── ergogen-outline/       # Outline generation
│   │   ├── shapes.rs          # Shape primitives
│   │   ├── operations.rs      # Boolean operations
│   │   └── expand.rs          # Expand/fillet
│   │
│   ├── ergogen-pcb/           # PCB generation
│   │   ├── footprint.rs       # Footprint loading
│   │   ├── nets.rs            # Net assignment
│   │   └── kicad.rs           # KiCAD format
│   │
│   ├── ergogen-export/        # File exports
│   │   ├── dxf.rs             # DXF writer
│   │   ├── svg.rs             # SVG writer
│   │   ├── jscad.rs           # JSCAD script generator
│   │   └── kicad.rs           # KiCAD PCB writer
│   │
│   └── ergogen-cli/           # CLI application
│       └── main.rs
│
├── ergogen-wasm/              # WebAssembly bindings
│   └── lib.rs
│
└── footprints/                # Declarative footprint definitions
    ├── mx.yaml
    ├── choc.yaml
    └── ...
```

### Key Data Structures

```rust
// crates/ergogen-core/src/point.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub r: f64,  // rotation in degrees
    pub meta: PointMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointMeta {
    pub name: String,
    pub zone: String,
    pub column: String,
    pub row: String,
    pub width: f64,
    pub height: f64,
    pub mirrored: bool,
    pub bind: [f64; 4],
    // ... other metadata
}

impl Point {
    pub fn shift(&mut self, delta: [f64; 2]) { /* ... */ }
    pub fn rotate(&mut self, angle: f64, origin: Option<[f64; 2]>) { /* ... */ }
    pub fn mirror(&mut self, axis: f64) { /* ... */ }
}
```

```rust
// crates/ergogen-geometry/src/primitives.rs
pub enum Shape {
    Rectangle { width: f64, height: f64, corner: f64 },
    Circle { radius: f64 },
    Polygon { points: Vec<[f64; 2]> },
    Path { segments: Vec<PathSegment> },
    Hull { points: Vec<[f64; 2]>, concave: bool },
}

pub enum PathSegment {
    Line { to: [f64; 2] },
    Arc { to: [f64; 2], radius: f64 },
    SCurve { to: [f64; 2] },
    Bezier { control1: [f64; 2], control2: [f64; 2], to: [f64; 2] },
}
```

### Dependency Mapping

| JavaScript | Rust Crate | Notes |
|------------|------------|-------|
| makerjs | kurbo + geo + i_overlay + cavalier_contours | Multiple crates needed |
| js-yaml | serde_yml | Direct replacement |
| mathjs | meval-rs or exmex | Expression evaluation |
| jszip | zip | Direct replacement |
| yargs | clap (derive) | CLI parsing |
| fs-extra | std::fs + walkdir | Standard library sufficient |
| hull | geo::algorithm::ConvexHull + custom | Concave hull needs implementation |
| kle-serial | Custom implementation | Port the conversion logic |

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-4)

**Goal:** Establish core types, parsing, and test infrastructure

**Tasks:**
- [ ] Set up Rust workspace with crate structure
- [ ] Implement Point struct with transformations
- [ ] Implement config parsing with serde_yml
- [ ] Port assert.js validation logic to Rust Result types
- [ ] Implement units system with expression evaluation
- [ ] Set up golden file test infrastructure
- [ ] Validate against first 5 simple point fixtures

**Key Files:**
- `crates/ergogen-core/src/point.rs`
- `crates/ergogen-parser/src/yaml.rs`
- `crates/ergogen-parser/src/units.rs`

**Success Criteria:**
- [ ] Parse basic YAML configs
- [ ] Create Point instances with correct coordinates
- [ ] Evaluate unit expressions (e.g., "u - 1")
- [ ] Pass 5 point generation tests

### Phase 2: Point Generation (Weeks 5-8)

**Goal:** Port the complex points layout algorithm

**Tasks:**
- [ ] Implement anchor resolution system
- [ ] Implement filter parsing and evaluation
- [ ] Port zone rendering with cumulative rotations
- [ ] Implement column/row key generation
- [ ] Port mirror axis calculation
- [ ] Implement autobind logic
- [ ] Handle 5-level config inheritance
- [ ] Validate against all 68 point fixtures

**Key Files:**
- `crates/ergogen-core/src/anchor.rs`
- `crates/ergogen-core/src/filter.rs`
- `crates/ergogen-layout/src/zones.rs`

**Success Criteria:**
- [ ] All point fixtures produce matching output
- [ ] Complex mirrors work correctly
- [ ] Autobind matches JavaScript behavior

### Phase 3: Geometry Engine (Weeks 9-14)

**Goal:** Build geometry primitives and operations

**Tasks:**
- [ ] Integrate kurbo for curves and paths
- [ ] Integrate geo for polygon operations
- [ ] Implement shape primitives (rectangle, circle, polygon)
- [ ] Port path segment types (line, arc, s_curve, bezier)
- [ ] Implement boolean operations (union, subtract, intersect)
- [ ] Implement expand/fillet with cavalier_contours
- [ ] Implement convex hull with geo
- [ ] Implement concave hull (port algorithm)
- [ ] Validate against outline fixtures

**Key Files:**
- `crates/ergogen-geometry/src/primitives.rs`
- `crates/ergogen-geometry/src/boolean.rs`
- `crates/ergogen-outline/src/shapes.rs`

**Success Criteria:**
- [ ] Boolean operations produce valid polygons
- [ ] Expand/fillet preserves arc quality
- [ ] All outline fixtures pass

### Phase 4: Export Formats (Weeks 15-18)

**Goal:** Generate output files in all formats

**Tasks:**
- [ ] Implement DXF export using dxf crate
- [ ] Implement SVG export (custom or svg crate)
- [ ] Port JSCAD script generation (string templating)
- [ ] Implement KiCAD PCB generation with templates
- [ ] Design declarative footprint format
- [ ] Convert built-in footprints to declarative format
- [ ] Implement footprint loader and renderer
- [ ] Validate output compatibility with downstream tools

**Key Files:**
- `crates/ergogen-export/src/dxf.rs`
- `crates/ergogen-export/src/svg.rs`
- `crates/ergogen-pcb/src/footprint.rs`
- `footprints/*.yaml`

**Success Criteria:**
- [ ] DXF files open correctly in laser cutter software
- [ ] SVG files render correctly in browsers
- [ ] KiCAD files open without errors in KiCAD 5/8
- [ ] JSCAD outputs generate valid 3D models

### Phase 5: CLI & I/O (Weeks 19-22)

**Goal:** Complete CLI with full feature parity

**Tasks:**
- [ ] Implement CLI with clap (derive macros)
- [ ] Port bundle (.ekb/.zip) handling
- [ ] Implement directory input mode
- [ ] Port KLE format detection and conversion
- [ ] Match exit codes (0=success, 1=usage, 2=input, 3=processing)
- [ ] Implement --debug, --clean, --svg flags
- [ ] End-to-end CLI testing

**Key Files:**
- `crates/ergogen-cli/src/main.rs`

**Success Criteria:**
- [ ] CLI passes all end-to-end tests
- [ ] Bundle extraction works
- [ ] KLE conversion produces valid configs

### Phase 6: WebAssembly & Polish (Weeks 23-26)

**Goal:** Browser deployment and final polish

**Tasks:**
- [ ] Set up wasm-pack build
- [ ] Create wasm-bindgen bindings
- [ ] Implement virtual filesystem for bundles
- [ ] Optimize bundle size (<500KB target)
- [ ] Integrate with existing web UI
- [ ] Performance benchmarking (target: 3x faster)
- [ ] Add parallelization with rayon where beneficial
- [ ] Write migration guide for users
- [ ] Document new footprint format

**Key Files:**
- `ergogen-wasm/src/lib.rs`
- `docs/migration-guide.md`

**Success Criteria:**
- [ ] WASM build works in ergogen.xyz
- [ ] 3x or better performance improvement
- [ ] Bundle size under 500KB
- [ ] Migration guide complete

---

## Acceptance Criteria

### Functional Requirements

- [ ] Parse all valid Ergogen 4.x YAML/JSON configs
- [ ] Generate identical point layouts (within floating-point tolerance)
- [ ] Generate semantically equivalent outlines
- [ ] Produce valid DXF files for laser cutting
- [ ] Produce valid SVG files for visualization
- [ ] Generate valid KiCAD 5 and 8 PCB files
- [ ] Generate valid JSCAD scripts for 3D cases
- [ ] Support all built-in footprints in new declarative format
- [ ] Handle bundles with custom footprints (declarative format)

### Non-Functional Requirements

- [ ] Performance: 3x faster than JavaScript version on typical configs
- [ ] Memory: Use less memory than Node.js version
- [ ] Binary size: Native CLI under 10MB
- [ ] WASM size: Under 500KB (gzipped)
- [ ] Test coverage: 90%+ code coverage
- [ ] Documentation: Rustdoc for all public APIs

### Quality Gates

- [x] All imported integration fixtures pass (64 YAML; 61 reference artifacts across points/outlines/cases/pcbs/footprints)
- [x] CLI integration fixtures (test/cli) are ported and passing (logs/errors + 73 reference artifacts)
- [ ] No unsafe code except for WASM bindings
- [ ] No clippy warnings
- [ ] Formatted with rustfmt
- [ ] Benchmarks show expected performance gains

---

## Risk Analysis & Mitigation

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| Boolean ops edge cases | High | Medium | Use well-tested libraries (i_overlay), extensive property testing |
| makerjs feature gaps | High | High | Early spike on geometry stack, have fallback plans |
| Breaking changes upset users | Medium | High | Clear migration guide, version 5.0, compatibility mode |
| WASM performance regression | Medium | Low | Benchmark early, optimize if needed |
| Floating-point precision issues | Medium | Medium | Use f64 throughout, epsilon comparisons in tests |
| Custom footprint migration | High | High | Provide migration tool, community support period |

---

## Dependencies & Prerequisites

**Required before starting:**
- [ ] Decision on JavaScript execution strategy (Recommended: Option C)
- [ ] Decision on footprint format specification
- [ ] Confirm acceptable output differences

**External dependencies:**
- Rust 1.70+ (for stable features)
- wasm-pack 0.12+ (for WASM builds)
- Node.js 18+ (for comparison testing)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test pass rate | 100% | All 68 fixtures pass |
| Performance improvement | 3x | Benchmark on reference keyboards |
| WASM bundle size | <500KB | gzipped size |
| Memory usage | <50% of JS | Peak memory on large configs |
| Community adoption | 50% | GitHub stars, Discord feedback |

---

## Future Considerations

**Post-1.0 features:**
- Plugin system with Rhai scripting
- Parallel outline generation with rayon
- GPU-accelerated geometry operations
- Native 3D case generation (skip JSCAD)
- Live preview server (file watching)
- Language server for config editing
- Integration with KiCAD directly

---

## References & Research

### Internal References
- Repository analysis: `/Users/fcoury/code/ergogen/ERGOGEN_REPOSITORY_ANALYSIS.md`
- Rust migration research: `/Users/fcoury/code/ergogen/RUST_MIGRATION_RESEARCH.md`
- Dependencies research: `/Users/fcoury/code/ergogen/RUST_DEPENDENCIES_RESEARCH.md`
- Current package.json: `/Users/fcoury/code/ergogen/package.json:1`
- Main entry point: `/Users/fcoury/code/ergogen/src/ergogen.js:1`
- Complex points algorithm: `/Users/fcoury/code/ergogen/src/points.js:1`
- Outlines with makerjs: `/Users/fcoury/code/ergogen/src/outlines.js:1`

### External References
- [kurbo docs](https://docs.rs/kurbo) - 2D curves and paths
- [geo docs](https://docs.rs/geo) - Geospatial primitives
- [i_overlay docs](https://docs.rs/i_overlay) - Polygon boolean operations
- [cavalier_contours docs](https://docs.rs/cavalier_contours) - Arc-aware polyline operations
- [clap docs](https://docs.rs/clap) - CLI argument parsing
- [serde_yml docs](https://docs.rs/serde_yml) - YAML parsing
- [wasm-pack guide](https://rustwasm.github.io/wasm-pack/)

### Related Work
- Ergogen Discord: http://discord.ergogen.xyz
- Ergogen docs: https://docs.ergogen.xyz
- makerjs source: https://github.com/microsoft/maker.js

---

## MVP

The minimum viable product is a CLI that can:
1. Parse YAML configs
2. Generate points
3. Generate outlines with basic shapes
4. Export DXF/SVG

This allows early validation of the geometry stack before tackling PCB generation.

### Cargo.toml (workspace root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/ergogen-core",
    "crates/ergogen-geometry",
    "crates/ergogen-parser",
    "crates/ergogen-layout",
    "crates/ergogen-outline",
    "crates/ergogen-pcb",
    "crates/ergogen-export",
    "crates/ergogen-cli",
    "ergogen-wasm",
]

[workspace.package]
version = "5.0.0-alpha.1"
edition = "2021"
license = "MIT"
repository = "https://github.com/ergogen/ergogen-rs"

[workspace.dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yml = "0.0.12"
thiserror = "2.0"
anyhow = "1.0"

# Geometry
kurbo = "0.11"
geo = "0.28"
i_overlay = "1.9"
# cavalier_contours = "0.3"  # Add when needed

# Math
meval = "0.2"

# CLI
clap = { version = "4.5", features = ["derive"] }

# Export
dxf = "0.6"
svg = "0.17"

# I/O
zip = "2.2"
walkdir = "2.5"

# Testing
approx = "0.5"
proptest = "1.5"
```

### crates/ergogen-core/src/point.rs

```rust
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    #[serde(default)]
    pub meta: PointMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PointMeta {
    pub name: String,
    pub zone: String,
    pub column: String,
    pub row: String,
    #[serde(default = "default_dimension")]
    pub width: f64,
    #[serde(default = "default_dimension")]
    pub height: f64,
    #[serde(default)]
    pub mirrored: bool,
    #[serde(default)]
    pub bind: [f64; 4],
}

fn default_dimension() -> f64 {
    1.0
}

impl Point {
    pub fn new(x: f64, y: f64, r: f64) -> Self {
        Self {
            x,
            y,
            r,
            meta: PointMeta::default(),
        }
    }

    pub fn shift(&mut self, delta: [f64; 2]) {
        let rad = self.r * PI / 180.0;
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        self.x += delta[0] * cos_r - delta[1] * sin_r;
        self.y += delta[0] * sin_r + delta[1] * cos_r;
    }

    pub fn rotate(&mut self, angle: f64, origin: Option<[f64; 2]>) {
        let origin = origin.unwrap_or([self.x, self.y]);
        let rad = angle * PI / 180.0;
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        let dx = self.x - origin[0];
        let dy = self.y - origin[1];

        self.x = origin[0] + dx * cos_a - dy * sin_a;
        self.y = origin[1] + dx * sin_a + dy * cos_a;
        self.r += angle;
    }

    pub fn mirror(&mut self, axis: f64) {
        self.x = 2.0 * axis - self.x;
        self.r = 180.0 - self.r;
        self.meta.mirrored = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_shift() {
        let mut p = Point::new(0.0, 0.0, 0.0);
        p.shift([10.0, 5.0]);
        assert_relative_eq!(p.x, 10.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_shift_with_rotation() {
        let mut p = Point::new(0.0, 0.0, 90.0);
        p.shift([10.0, 0.0]);
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rotate() {
        let mut p = Point::new(10.0, 0.0, 0.0);
        p.rotate(90.0, Some([0.0, 0.0]));
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 10.0, epsilon = 1e-10);
        assert_relative_eq!(p.r, 90.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mirror() {
        let mut p = Point::new(10.0, 5.0, 45.0);
        p.mirror(0.0);
        assert_relative_eq!(p.x, -10.0, epsilon = 1e-10);
        assert_relative_eq!(p.r, 135.0, epsilon = 1e-10);
        assert!(p.meta.mirrored);
    }
}
```

---

**Generated with Claude Code**

**Estimated Duration:** 6-7 months (26 weeks) with 1-2 developers

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DxfError {
    #[error("DXF I/O error for {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("DXF parse error: expected an even number of lines (code/value pairs)")]
    OddNumberOfLines,
    #[error("DXF parse error: invalid group code {raw:?} at line {line}")]
    InvalidGroupCode { raw: String, line: usize },
    #[error("DXF parse error: missing ENTITIES section")]
    MissingEntitiesSection,
    #[error("DXF parse error: unexpected end of file while parsing {context}")]
    UnexpectedEof { context: &'static str },
    #[error("DXF parse error: missing required group code {code} for entity {entity}")]
    MissingRequiredGroup { entity: &'static str, code: i32 },
    #[error("DXF parse error: invalid float {raw:?} for group code {code} in entity {entity}")]
    InvalidFloat {
        entity: &'static str,
        code: i32,
        raw: String,
    },
    #[error("DXF normalize error: eps must be > 0 (got {eps})")]
    InvalidEpsilon { eps: f64 },
    #[error("DXF normalize error: non-finite value for {what}")]
    NonFinite { what: &'static str },
    #[error("DXF normalize error: quantized value out of i64 range for {what}")]
    QuantizeOutOfRange { what: &'static str },
    #[error("DXF contains unsupported entities: {kinds:?}")]
    UnsupportedEntities { kinds: Vec<String> },
}

#[derive(Debug, Clone, Copy)]
pub struct NormalizeOptions {
    pub linear_eps: f64,
    pub angle_eps_deg: f64,
    pub allow_unsupported: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            linear_eps: 1e-6,
            angle_eps_deg: 1e-6,
            allow_unsupported: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Dxf {
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone)]
pub enum Entity {
    Line(Line),
    Circle(Circle),
    Arc(Arc),
    LwPolyline(LwPolyline),
    Unsupported(Unsupported),
}

#[derive(Debug, Clone)]
pub struct Unsupported {
    pub kind: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub start: Point2,
    pub end: Point2,
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Point2,
    pub radius: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Arc {
    pub center: Point2,
    pub radius: f64,
    pub start_angle_deg: f64,
    pub end_angle_deg: f64,
}

#[derive(Debug, Clone)]
pub struct LwPolyline {
    pub vertices: Vec<Point2>,
    pub bulges: Vec<f64>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy)]
struct Group<'a> {
    code: i32,
    value: &'a str,
}

#[derive(Debug, Clone)]
pub struct NormalizedDxf {
    pub entities: Vec<NormalizedEntity>,
}

#[derive(Debug, thiserror::Error)]
pub enum DxfCompareError {
    #[error("DXF entity count differs: left={left} right={right}")]
    EntityCount { left: usize, right: usize },
    #[error("DXF entity mismatch at index {index}: left={left:?} right={right:?}")]
    EntityMismatch {
        index: usize,
        left: NormalizedEntity,
        right: NormalizedEntity,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum DxfSemanticError {
    #[error("left DXF failed: {0}")]
    Left(#[source] DxfError),
    #[error("right DXF failed: {0}")]
    Right(#[source] DxfError),
    #[error("DXF semantic mismatch: {0}")]
    Mismatch(#[source] DxfCompareError),
}

#[derive(Debug, thiserror::Error)]
pub enum DxfWriteError {
    #[error("DXF write error: linear_eps must be > 0 (got {eps})")]
    InvalidLinearEpsilon { eps: f64 },
    #[error("DXF write error: angle_eps_deg must be > 0 (got {eps})")]
    InvalidAngleEpsilon { eps: f64 },
    #[error("DXF write error: non-finite dequantized value for {what}")]
    NonFinite { what: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NormalizedEntity {
    Line {
        a: QPoint2,
        b: QPoint2,
    },
    Circle {
        c: QPoint2,
        r: i64,
    },
    Arc {
        c: QPoint2,
        r: i64,
        start: i64,
        end: i64,
    },
    Polyline {
        vertices: Vec<QPoint2>,
        bulges: Vec<i64>,
        closed: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QPoint2 {
    pub x: i64,
    pub y: i64,
}

impl NormalizedDxf {
    pub fn compare_semantic(&self, other: &Self) -> Result<(), DxfCompareError> {
        if self.entities.len() != other.entities.len() {
            return Err(DxfCompareError::EntityCount {
                left: self.entities.len(),
                right: other.entities.len(),
            });
        }

        for (idx, (l, r)) in self
            .entities
            .iter()
            .cloned()
            .zip(other.entities.iter().cloned())
            .enumerate()
        {
            if l != r {
                return Err(DxfCompareError::EntityMismatch {
                    index: idx,
                    left: l,
                    right: r,
                });
            }
        }

        Ok(())
    }

    pub fn to_dxf_string(&self, opts: NormalizeOptions) -> Result<String, DxfWriteError> {
        if opts.linear_eps <= 0.0 {
            return Err(DxfWriteError::InvalidLinearEpsilon {
                eps: opts.linear_eps,
            });
        }
        if opts.angle_eps_deg <= 0.0 {
            return Err(DxfWriteError::InvalidAngleEpsilon {
                eps: opts.angle_eps_deg,
            });
        }

        let mut out = String::new();
        // Minimal header + tables matching upstream Ergogen DXFs for maximum viewer compatibility.
        push_pair(&mut out, 0, "SECTION");
        push_pair(&mut out, 2, "HEADER");
        push_pair(&mut out, 9, "$INSUNITS");
        push_pair(&mut out, 70, "4"); // millimeters
        push_pair(&mut out, 0, "ENDSEC");
        push_pair(&mut out, 0, "SECTION");
        push_pair(&mut out, 2, "TABLES");
        push_pair(&mut out, 0, "TABLE");
        push_pair(&mut out, 2, "LTYPE");
        push_pair(&mut out, 0, "LTYPE");
        push_pair(&mut out, 72, "65");
        push_pair(&mut out, 70, "64");
        push_pair(&mut out, 2, "CONTINUOUS");
        push_pair(&mut out, 3, "______");
        push_pair(&mut out, 73, "0");
        push_pair(&mut out, 40, "0");
        push_pair(&mut out, 0, "ENDTAB");
        push_pair(&mut out, 0, "TABLE");
        push_pair(&mut out, 2, "LAYER");
        push_pair(&mut out, 0, "ENDTAB");
        push_pair(&mut out, 0, "ENDSEC");
        push_pair(&mut out, 0, "SECTION");
        push_pair(&mut out, 2, "ENTITIES");

        for e in &self.entities {
            match e {
                NormalizedEntity::Line { a, b } => {
                    push_pair(&mut out, 0, "LINE");
                    push_pair(&mut out, 8, "0");
                    push_pair_f64(
                        &mut out,
                        10,
                        dequantize_i64(a.x, opts.linear_eps, "line x1")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        20,
                        dequantize_i64(a.y, opts.linear_eps, "line y1")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        11,
                        dequantize_i64(b.x, opts.linear_eps, "line x2")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        21,
                        dequantize_i64(b.y, opts.linear_eps, "line y2")?,
                    )?;
                }
                NormalizedEntity::Circle { c, r } => {
                    push_pair(&mut out, 0, "CIRCLE");
                    push_pair(&mut out, 8, "0");
                    push_pair_f64(
                        &mut out,
                        10,
                        dequantize_i64(c.x, opts.linear_eps, "circle cx")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        20,
                        dequantize_i64(c.y, opts.linear_eps, "circle cy")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        40,
                        dequantize_i64(*r, opts.linear_eps, "circle r")?,
                    )?;
                }
                NormalizedEntity::Arc { c, r, start, end } => {
                    push_pair(&mut out, 0, "ARC");
                    push_pair(&mut out, 8, "0");
                    push_pair_f64(
                        &mut out,
                        10,
                        dequantize_i64(c.x, opts.linear_eps, "arc cx")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        20,
                        dequantize_i64(c.y, opts.linear_eps, "arc cy")?,
                    )?;
                    push_pair_f64(&mut out, 40, dequantize_i64(*r, opts.linear_eps, "arc r")?)?;
                    push_pair_f64(
                        &mut out,
                        50,
                        dequantize_i64(*start, opts.angle_eps_deg, "arc start")?,
                    )?;
                    push_pair_f64(
                        &mut out,
                        51,
                        dequantize_i64(*end, opts.angle_eps_deg, "arc end")?,
                    )?;
                }
                NormalizedEntity::Polyline {
                    vertices,
                    bulges,
                    closed,
                } => {
                    push_pair(&mut out, 0, "LWPOLYLINE");
                    push_pair(&mut out, 8, "0");
                    push_pair(&mut out, 90, vertices.len().to_string());
                    push_pair(&mut out, 70, if *closed { "1" } else { "0" });
                    for (idx, v) in vertices.iter().enumerate() {
                        push_pair_f64(
                            &mut out,
                            10,
                            dequantize_i64(v.x, opts.linear_eps, "polyline x")?,
                        )?;
                        push_pair_f64(
                            &mut out,
                            20,
                            dequantize_i64(v.y, opts.linear_eps, "polyline y")?,
                        )?;
                        let bulge = bulges.get(idx).copied().unwrap_or(0);
                        push_pair_f64(
                            &mut out,
                            42,
                            dequantize_i64(bulge, opts.linear_eps, "polyline bulge")?,
                        )?;
                    }
                }
            }
        }

        push_pair(&mut out, 0, "ENDSEC");
        push_pair(&mut out, 0, "EOF");
        Ok(out)
    }
}

pub fn compare_files_semantic(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
    opts: NormalizeOptions,
) -> Result<(), DxfSemanticError> {
    let left = Dxf::parse_file(left).map_err(DxfSemanticError::Left)?;
    let right = Dxf::parse_file(right).map_err(DxfSemanticError::Right)?;
    let left = left.normalize(opts).map_err(DxfSemanticError::Left)?;
    let right = right.normalize(opts).map_err(DxfSemanticError::Right)?;
    left.compare_semantic(&right)
        .map_err(DxfSemanticError::Mismatch)?;
    Ok(())
}

fn dequantize_i64(q: i64, eps: f64, what: &'static str) -> Result<f64, DxfWriteError> {
    let v = (q as f64) * eps;
    if !v.is_finite() {
        return Err(DxfWriteError::NonFinite { what });
    }
    Ok(v)
}

fn push_pair(out: &mut String, code: i32, value: impl AsRef<str>) {
    out.push_str(&code.to_string());
    out.push('\n');
    out.push_str(value.as_ref());
    out.push('\n');
}

fn push_pair_f64(out: &mut String, code: i32, value: f64) -> Result<(), DxfWriteError> {
    if !value.is_finite() {
        return Err(DxfWriteError::NonFinite { what: "float" });
    }
    let mut buf = ryu::Buffer::new();
    push_pair(out, code, buf.format(value));
    Ok(())
}

impl Dxf {
    pub fn parse_str(input: &str) -> Result<Self, DxfError> {
        let groups = parse_groups(input)?;
        let entities = parse_entities(&groups)?;
        Ok(Self { entities })
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self, DxfError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|e| DxfError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::parse_str(&raw)
    }

    pub fn normalize(&self, opts: NormalizeOptions) -> Result<NormalizedDxf, DxfError> {
        if opts.linear_eps <= 0.0 {
            return Err(DxfError::InvalidEpsilon {
                eps: opts.linear_eps,
            });
        }
        if opts.angle_eps_deg <= 0.0 {
            return Err(DxfError::InvalidEpsilon {
                eps: opts.angle_eps_deg,
            });
        }

        let mut unsupported: Vec<String> = Vec::new();
        let mut out: Vec<NormalizedEntity> = Vec::new();

        for e in &self.entities {
            match e {
                Entity::Line(l) => {
                    let a = quantize_point(l.start, opts.linear_eps)?;
                    let b = quantize_point(l.end, opts.linear_eps)?;
                    let (a, b) = if a <= b { (a, b) } else { (b, a) };
                    out.push(NormalizedEntity::Line { a, b });
                }
                Entity::Circle(c) => {
                    let center = quantize_point(c.center, opts.linear_eps)?;
                    let r = quantize(c.radius, opts.linear_eps, "circle radius")?;
                    out.push(NormalizedEntity::Circle { c: center, r });
                }
                Entity::Arc(a) => {
                    let center = quantize_point(a.center, opts.linear_eps)?;
                    let r = quantize(a.radius, opts.linear_eps, "arc radius")?;
                    let start = quantize(
                        norm_deg(a.start_angle_deg),
                        opts.angle_eps_deg,
                        "arc start angle",
                    )?;
                    let end = quantize(
                        norm_deg(a.end_angle_deg),
                        opts.angle_eps_deg,
                        "arc end angle",
                    )?;
                    out.push(NormalizedEntity::Arc {
                        c: center,
                        r,
                        start,
                        end,
                    });
                }
                Entity::LwPolyline(p) => {
                    let mut vertices: Vec<QPoint2> = Vec::with_capacity(p.vertices.len());
                    for v in &p.vertices {
                        vertices.push(quantize_point(*v, opts.linear_eps)?);
                    }
                    let mut bulges: Vec<i64> = Vec::with_capacity(p.bulges.len());
                    for b in &p.bulges {
                        bulges.push(quantize(*b, opts.linear_eps, "polyline bulge")?);
                    }
                    canonicalize_polyline(&mut vertices, &mut bulges, p.closed);
                    out.push(NormalizedEntity::Polyline {
                        vertices,
                        bulges,
                        closed: p.closed,
                    });
                }
                Entity::Unsupported(u) => {
                    unsupported.push(u.kind.clone());
                }
            }
        }

        if !opts.allow_unsupported && !unsupported.is_empty() {
            unsupported.sort();
            unsupported.dedup();
            return Err(DxfError::UnsupportedEntities { kinds: unsupported });
        }

        out.sort();
        Ok(NormalizedDxf { entities: out })
    }
}

fn parse_groups(input: &str) -> Result<Vec<Group<'_>>, DxfError> {
    let lines: Vec<&str> = input.lines().collect();
    if !lines.len().is_multiple_of(2) {
        return Err(DxfError::OddNumberOfLines);
    }

    let mut groups: Vec<Group<'_>> = Vec::with_capacity(lines.len() / 2);
    let mut i = 0usize;
    while i < lines.len() {
        let code_line = i + 1; // 1-based for errors
        let code_raw = lines[i].trim();
        let value = lines[i + 1].trim_end();
        let code: i32 = code_raw.parse().map_err(|_| DxfError::InvalidGroupCode {
            raw: code_raw.to_string(),
            line: code_line,
        })?;
        groups.push(Group { code, value });
        i += 2;
    }
    Ok(groups)
}

fn parse_entities(groups: &[Group<'_>]) -> Result<Vec<Entity>, DxfError> {
    let mut i = 0usize;
    let mut in_entities = false;
    let mut saw_entities_section = false;
    let mut entities: Vec<Entity> = Vec::new();

    while i < groups.len() {
        let g = groups[i];
        if g.code == 0 && g.value == "SECTION" {
            i += 1;
            let Some(name_g) = groups.get(i) else {
                return Err(DxfError::UnexpectedEof {
                    context: "SECTION name",
                });
            };
            if name_g.code == 2 && name_g.value == "ENTITIES" {
                in_entities = true;
                saw_entities_section = true;
            }
            i += 1;
            continue;
        }

        if in_entities {
            if g.code == 0 && g.value == "ENDSEC" {
                in_entities = false;
                i += 1;
                continue;
            }

            if g.code == 0 {
                let kind = g.value;
                i += 1;
                let start = i;

                // Most entities run until the next 0-group (next entity marker).
                // POLYLINE is special: it contains nested 0-groups (VERTEX/SEQEND).
                if kind == "POLYLINE" {
                    while i < groups.len() {
                        if groups[i].code == 0 && groups[i].value == "SEQEND" {
                            i += 1;
                            while i < groups.len() && groups[i].code != 0 {
                                i += 1;
                            }
                            break;
                        }
                        i += 1;
                    }
                } else {
                    while i < groups.len() && groups[i].code != 0 {
                        i += 1;
                    }
                }

                let slice = &groups[start..i];
                entities.push(parse_entity(kind, slice)?);
                continue;
            }
        }

        i += 1;
    }

    if saw_entities_section {
        return Ok(entities);
    }

    Err(DxfError::MissingEntitiesSection)
}

fn parse_entity(kind: &str, groups: &[Group<'_>]) -> Result<Entity, DxfError> {
    match kind {
        "LINE" => Ok(Entity::Line(parse_line(groups)?)),
        "CIRCLE" => Ok(Entity::Circle(parse_circle(groups)?)),
        "ARC" => Ok(Entity::Arc(parse_arc(groups)?)),
        "LWPOLYLINE" => Ok(Entity::LwPolyline(parse_lwpolyline(groups)?)),
        other => Ok(Entity::Unsupported(Unsupported {
            kind: other.to_string(),
        })),
    }
}

fn parse_line(groups: &[Group<'_>]) -> Result<Line, DxfError> {
    let entity = "LINE";
    let x1 = get_f64(entity, groups, 10)?;
    let y1 = get_f64(entity, groups, 20)?;
    let x2 = get_f64(entity, groups, 11)?;
    let y2 = get_f64(entity, groups, 21)?;
    Ok(Line {
        start: Point2 { x: x1, y: y1 },
        end: Point2 { x: x2, y: y2 },
    })
}

fn parse_circle(groups: &[Group<'_>]) -> Result<Circle, DxfError> {
    let entity = "CIRCLE";
    let x = get_f64(entity, groups, 10)?;
    let y = get_f64(entity, groups, 20)?;
    let r = get_f64(entity, groups, 40)?;
    Ok(Circle {
        center: Point2 { x, y },
        radius: r,
    })
}

fn parse_arc(groups: &[Group<'_>]) -> Result<Arc, DxfError> {
    let entity = "ARC";
    let x = get_f64(entity, groups, 10)?;
    let y = get_f64(entity, groups, 20)?;
    let r = get_f64(entity, groups, 40)?;
    let start = get_f64(entity, groups, 50)?;
    let end = get_f64(entity, groups, 51)?;
    Ok(Arc {
        center: Point2 { x, y },
        radius: r,
        start_angle_deg: start,
        end_angle_deg: end,
    })
}

fn parse_lwpolyline(groups: &[Group<'_>]) -> Result<LwPolyline, DxfError> {
    let mut closed = false;
    let mut vertices: Vec<Point2> = Vec::new();
    let mut bulges: Vec<f64> = Vec::new();

    for g in groups {
        if g.code == 70
            && let Ok(flags) = g.value.trim().parse::<i32>()
        {
            closed = (flags & 1) != 0;
        }
    }

    let mut last_x: Option<f64> = None;
    for g in groups {
        match g.code {
            10 => {
                let x = parse_f64("LWPOLYLINE", 10, g.value)?;
                last_x = Some(x);
            }
            20 => {
                let Some(x) = last_x.take() else {
                    continue;
                };
                let y = parse_f64("LWPOLYLINE", 20, g.value)?;
                vertices.push(Point2 { x, y });
                bulges.push(0.0);
            }
            42 => {
                if let Some(last) = bulges.last_mut() {
                    *last = parse_f64("LWPOLYLINE", 42, g.value)?;
                }
            }
            _ => {}
        }
    }

    Ok(LwPolyline {
        vertices,
        bulges,
        closed,
    })
}

fn get_f64(entity: &'static str, groups: &[Group<'_>], code: i32) -> Result<f64, DxfError> {
    let Some(g) = groups.iter().find(|g| g.code == code) else {
        return Err(DxfError::MissingRequiredGroup { entity, code });
    };
    parse_f64(entity, code, g.value)
}

fn parse_f64(entity: &'static str, code: i32, raw: &str) -> Result<f64, DxfError> {
    raw.trim()
        .parse::<f64>()
        .map_err(|_| DxfError::InvalidFloat {
            entity,
            code,
            raw: raw.to_string(),
        })
}

fn quantize_point(p: Point2, eps: f64) -> Result<QPoint2, DxfError> {
    Ok(QPoint2 {
        x: quantize(p.x, eps, "x")?,
        y: quantize(p.y, eps, "y")?,
    })
}

fn quantize(value: f64, eps: f64, what: &'static str) -> Result<i64, DxfError> {
    if !value.is_finite() {
        return Err(DxfError::NonFinite { what });
    }
    let scaled = value / eps;
    if !scaled.is_finite() {
        return Err(DxfError::NonFinite { what });
    }
    if scaled.abs() > (i64::MAX as f64) {
        return Err(DxfError::QuantizeOutOfRange { what });
    }
    Ok(scaled.round() as i64)
}

fn norm_deg(v: f64) -> f64 {
    // DXF angles are degrees; normalize to [0, 360).
    v.rem_euclid(360.0)
}

fn canonicalize_polyline(vertices: &mut Vec<QPoint2>, bulges: &mut Vec<i64>, closed: bool) {
    if vertices.is_empty() {
        return;
    }
    if vertices.len() == 1 {
        bulges.truncate(1);
        return;
    }

    // Ensure bulges length matches vertices length.
    if bulges.len() < vertices.len() {
        bulges.resize(vertices.len(), 0);
    }
    if bulges.len() > vertices.len() {
        bulges.truncate(vertices.len());
    }

    let mut best_vertices = vertices.clone();
    let mut best_bulges = bulges.clone();

    let candidates = if closed { vertices.len() } else { 1 };

    for rot in 0..candidates {
        let (v1, b1) = rotated(vertices, bulges, rot);
        let (v2, b2) = reversed(&v1, &b1, closed);

        if (v1.as_slice(), b1.as_slice()) < (best_vertices.as_slice(), best_bulges.as_slice()) {
            best_vertices = v1;
            best_bulges = b1;
        }
        if (v2.as_slice(), b2.as_slice()) < (best_vertices.as_slice(), best_bulges.as_slice()) {
            best_vertices = v2;
            best_bulges = b2;
        }
    }

    *vertices = best_vertices;
    *bulges = best_bulges;
}

fn rotated(vertices: &[QPoint2], bulges: &[i64], rot: usize) -> (Vec<QPoint2>, Vec<i64>) {
    let mut v: Vec<QPoint2> = Vec::with_capacity(vertices.len());
    let mut b: Vec<i64> = Vec::with_capacity(vertices.len());

    for i in 0..vertices.len() {
        let idx = (i + rot) % vertices.len();
        v.push(vertices[idx]);
        b.push(bulges[idx]);
    }

    (v, b)
}

fn reversed(vertices: &[QPoint2], bulges: &[i64], closed: bool) -> (Vec<QPoint2>, Vec<i64>) {
    let v: Vec<QPoint2> = vertices.iter().copied().rev().collect();
    let mut b: Vec<i64> = bulges.iter().copied().rev().collect();

    if closed {
        // Reversal changes which segment a bulge applies to; for closed polylines we can
        // rotate bulges by one to keep per-vertex association stable-ish.
        if let Some(last) = b.pop() {
            b.insert(0, last);
        }
    }

    // Keep lengths aligned.
    b.resize(v.len(), 0);
    (v, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_makes_line_direction_irrelevant() {
        let a = r#"0
SECTION
2
ENTITIES
0
LINE
10
0
20
0
11
1
21
0
0
ENDSEC
0
EOF
"#;

        let b = r#"0
SECTION
2
ENTITIES
0
LINE
10
1
20
0
11
0
21
0
0
ENDSEC
0
EOF
"#;

        let left = Dxf::parse_str(a)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        let right = Dxf::parse_str(b)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        left.compare_semantic(&right).unwrap();
    }

    #[test]
    fn normalization_ignores_entity_order() {
        let a = r#"0
SECTION
2
ENTITIES
0
LINE
10
0
20
0
11
1
21
0
0
CIRCLE
10
5
20
6
40
7
0
ENDSEC
0
EOF
"#;

        let b = r#"0
SECTION
2
ENTITIES
0
CIRCLE
10
5
20
6
40
7
0
LINE
10
1
20
0
11
0
21
0
0
ENDSEC
0
EOF
"#;

        let left = Dxf::parse_str(a)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        let right = Dxf::parse_str(b)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        left.compare_semantic(&right).unwrap();
    }

    #[test]
    fn normalization_normalizes_negative_angles() {
        let a = r#"0
SECTION
2
ENTITIES
0
ARC
10
0
20
0
40
1
50
-10
51
10
0
ENDSEC
0
EOF
"#;

        let b = r#"0
SECTION
2
ENTITIES
0
ARC
10
0
20
0
40
1
50
350
51
10
0
ENDSEC
0
EOF
"#;

        let left = Dxf::parse_str(a)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        let right = Dxf::parse_str(b)
            .unwrap()
            .normalize(NormalizeOptions::default())
            .unwrap();
        left.compare_semantic(&right).unwrap();
    }
}

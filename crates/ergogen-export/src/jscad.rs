use std::collections::{HashMap, HashSet};

use ergogen_parser::{PreparedConfig, Units, Value};
use indexmap::IndexMap;

#[derive(Debug, thiserror::Error)]
pub enum JscadError {
    #[error("\"cases\" section is missing")]
    MissingCases,
    #[error("\"outlines\" section is missing")]
    MissingOutlines,
    #[error("cases must be an object")]
    CasesNotMap,
    #[error("outlines must be an object")]
    OutlinesNotMap,
    #[error("unknown case \"{name}\"")]
    UnknownCase { name: String },
    #[error("unknown outline \"{name}\"")]
    UnknownOutline { name: String },
    #[error("unsupported outline shape for \"{name}\"")]
    UnsupportedOutline { name: String },
    #[error("invalid number at \"{at}\"")]
    InvalidNumber { at: String },
    #[error("invalid vector at \"{at}\"")]
    InvalidVector { at: String },
    #[error("invalid case definition for \"{name}\"")]
    InvalidCase { name: String },
    #[error("invalid case part for \"{name}\"")]
    InvalidCasePart { name: String },
}

#[derive(Debug, Clone, Copy)]
enum CaseOp {
    Union,
    Subtract,
    Intersect,
}

impl CaseOp {
    fn from_prefix(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Union),
            '-' => Some(Self::Subtract),
            '~' => Some(Self::Intersect),
            _ => None,
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "union" => Some(Self::Union),
            "subtract" => Some(Self::Subtract),
            "intersect" => Some(Self::Intersect),
            _ => None,
        }
    }

    fn js_method(self) -> &'static str {
        match self {
            Self::Union => "union",
            Self::Subtract => "subtract",
            Self::Intersect => "intersect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartWhat {
    Outline,
    Case,
}

#[derive(Debug, Clone)]
struct CasePart {
    name: String,
    what: PartWhat,
    extrude: f64,
    shift: [f64; 3],
    rotate: [f64; 3],
    operation: Option<CaseOp>,
}

#[derive(Debug, Clone)]
enum CaseDef {
    Parts(Vec<CasePart>),
    Op { target: CasePart, tool: CasePart },
}

#[derive(Debug, Clone)]
enum OutlineShape {
    Rectangle { w: f64, h: f64 },
    Circle { r: f64 },
    Region(ergogen_geometry::region::Region),
}

pub fn generate_cases_jscad(
    prepared: &PreparedConfig,
    case_name: &str,
) -> Result<String, JscadError> {
    let data = collect_case_data(prepared, case_name)?;
    render_cases_v1(case_name, data)
}

pub fn generate_cases_jscad_v2(
    prepared: &PreparedConfig,
    case_name: &str,
) -> Result<String, JscadError> {
    let data = collect_case_data(prepared, case_name)?;
    render_cases_v2(case_name, data)
}

struct CaseData {
    cases: IndexMap<String, CaseDef>,
    order: Vec<String>,
    outlines: Vec<(String, f64)>,
    outline_shapes: HashMap<String, OutlineShape>,
}

fn collect_case_data(prepared: &PreparedConfig, case_name: &str) -> Result<CaseData, JscadError> {
    let cases_v = prepared
        .canonical
        .get_path("cases")
        .ok_or(JscadError::MissingCases)?;
    let Value::Map(cases_map) = cases_v else {
        return Err(JscadError::CasesNotMap);
    };

    let outlines_v = prepared
        .canonical
        .get_path("outlines")
        .ok_or(JscadError::MissingOutlines)?;
    let Value::Map(outlines_map) = outlines_v else {
        return Err(JscadError::OutlinesNotMap);
    };

    let outline_names: HashSet<String> = outlines_map.keys().cloned().collect();

    let mut cases: IndexMap<String, CaseDef> = IndexMap::new();
    for (name, def_v) in cases_map {
        let def = parse_case_def(name, def_v, cases_map, &outline_names, &prepared.units)?;
        cases.insert(name.clone(), def);
    }

    if !cases.contains_key(case_name) {
        return Err(JscadError::UnknownCase {
            name: case_name.to_string(),
        });
    }

    let order = collect_case_order(case_name, &cases)?;
    let outlines = collect_outline_functions(&order, &cases)?;

    let mut outline_shapes: HashMap<String, OutlineShape> = HashMap::new();
    for (outline_name, _) in &outlines {
        if outline_shapes.contains_key(outline_name) {
            continue;
        }
        let shape = parse_outline_shape(outline_name, outlines_map, prepared)?;
        outline_shapes.insert(outline_name.clone(), shape);
    }

    Ok(CaseData {
        cases,
        order,
        outlines,
        outline_shapes,
    })
}

fn parse_case_def(
    name: &str,
    v: &Value,
    cases_map: &IndexMap<String, Value>,
    outline_names: &HashSet<String>,
    units: &Units,
) -> Result<CaseDef, JscadError> {
    match v {
        Value::Seq(seq) => {
            let mut parts = Vec::with_capacity(seq.len());
            for part in seq {
                parts.push(parse_case_part(
                    name,
                    part,
                    cases_map,
                    outline_names,
                    units,
                )?);
            }
            Ok(CaseDef::Parts(parts))
        }
        Value::Map(map) if map.contains_key("target") && map.contains_key("tool") => {
            let target_v = map.get("target").ok_or_else(|| JscadError::InvalidCase {
                name: name.to_string(),
            })?;
            let tool_v = map.get("tool").ok_or_else(|| JscadError::InvalidCase {
                name: name.to_string(),
            })?;
            let target = parse_case_part(name, target_v, cases_map, outline_names, units)?;
            let tool = parse_case_part(name, tool_v, cases_map, outline_names, units)?;
            Ok(CaseDef::Op { target, tool })
        }
        Value::Map(_) | Value::String(_) => {
            let part = parse_case_part(name, v, cases_map, outline_names, units)?;
            Ok(CaseDef::Parts(vec![part]))
        }
        _ => Err(JscadError::InvalidCase {
            name: name.to_string(),
        }),
    }
}

fn parse_case_part(
    case_name: &str,
    v: &Value,
    cases_map: &IndexMap<String, Value>,
    outline_names: &HashSet<String>,
    units: &Units,
) -> Result<CasePart, JscadError> {
    match v {
        Value::String(s) => parse_case_part_str(case_name, s, cases_map, outline_names),
        Value::Map(map) => {
            let name_v = map.get("name").ok_or_else(|| JscadError::InvalidCasePart {
                name: case_name.to_string(),
            })?;
            let Value::String(name) = name_v else {
                return Err(JscadError::InvalidCasePart {
                    name: case_name.to_string(),
                });
            };
            let what = map
                .get("what")
                .and_then(value_as_str)
                .and_then(|s| match s {
                    "case" => Some(PartWhat::Case),
                    "outline" => Some(PartWhat::Outline),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    if outline_names.contains(name) {
                        PartWhat::Outline
                    } else if cases_map.contains_key(name) {
                        PartWhat::Case
                    } else {
                        PartWhat::Outline
                    }
                });
            let extrude = map
                .get("extrude")
                .map(|v| parse_number(units, v, &format!("cases.{case_name}.extrude")))
                .transpose()?
                .unwrap_or_else(|| if what == PartWhat::Outline { 1.0 } else { 0.0 });
            let shift = parse_vec3(units, map.get("shift"), &format!("cases.{case_name}.shift"))?;
            let rotate = parse_vec3(
                units,
                map.get("rotate"),
                &format!("cases.{case_name}.rotate"),
            )?;
            let operation = map
                .get("operation")
                .and_then(value_as_str)
                .and_then(CaseOp::from_str);
            Ok(CasePart {
                name: name.clone(),
                what,
                extrude,
                shift,
                rotate,
                operation,
            })
        }
        _ => Err(JscadError::InvalidCasePart {
            name: case_name.to_string(),
        }),
    }
}

fn parse_case_part_str(
    _case_name: &str,
    raw: &str,
    cases_map: &IndexMap<String, Value>,
    outline_names: &HashSet<String>,
) -> Result<CasePart, JscadError> {
    let mut chars = raw.chars();
    let mut operation = None;
    let name = if let Some(first) = chars.next() {
        if let Some(op) = CaseOp::from_prefix(first) {
            operation = Some(op);
            chars.collect::<String>()
        } else {
            raw.to_string()
        }
    } else {
        raw.to_string()
    };
    let what = if outline_names.contains(&name) {
        PartWhat::Outline
    } else if cases_map.contains_key(&name) {
        PartWhat::Case
    } else {
        PartWhat::Outline
    };
    let extrude = if what == PartWhat::Outline { 1.0 } else { 0.0 };
    Ok(CasePart {
        name,
        what,
        extrude,
        shift: [0.0, 0.0, 0.0],
        rotate: [0.0, 0.0, 0.0],
        operation,
    })
}

fn collect_case_order(
    target: &str,
    cases: &IndexMap<String, CaseDef>,
) -> Result<Vec<String>, JscadError> {
    let mut order = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    fn visit(
        name: &str,
        is_root: bool,
        cases: &IndexMap<String, CaseDef>,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), JscadError> {
        if !visited.insert(name.to_string()) {
            return Ok(());
        }
        let def = cases.get(name).ok_or_else(|| JscadError::UnknownCase {
            name: name.to_string(),
        })?;
        if !is_root {
            order.push(name.to_string());
        }
        for part in case_parts(def) {
            if part.what == PartWhat::Case {
                visit(&part.name, false, cases, visited, order)?;
            }
        }
        Ok(())
    }

    visit(target, true, cases, &mut visited, &mut order)?;
    order.push(target.to_string());
    Ok(order)
}

fn collect_outline_functions(
    order: &[String],
    cases: &IndexMap<String, CaseDef>,
) -> Result<Vec<(String, f64)>, JscadError> {
    let mut out: Vec<(String, f64)> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for name in order {
        let def = cases
            .get(name)
            .ok_or_else(|| JscadError::UnknownCase { name: name.clone() })?;
        for part in case_parts(def) {
            if part.what != PartWhat::Outline {
                continue;
            }
            let key = (part.name.clone(), fmt_num(part.extrude));
            if seen.insert(key.clone()) {
                out.push((key.0, part.extrude));
            }
        }
    }

    Ok(out)
}

fn case_parts(def: &CaseDef) -> Vec<CasePart> {
    match def {
        CaseDef::Parts(parts) => parts.clone(),
        CaseDef::Op { target, tool } => vec![target.clone(), tool.clone()],
    }
}

fn render_case_fn(name: &str, def: &CaseDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("                function {name}_case_fn() {{\n"));
    out.push_str("                    \n");
    out.push('\n');

    let parts = match def {
        CaseDef::Parts(parts) => parts
            .iter()
            .cloned()
            .enumerate()
            .map(|(idx, part)| (idx.to_string(), part))
            .collect::<Vec<_>>(),
        CaseDef::Op { target, tool } => vec![
            ("target".to_string(), target.clone()),
            ("tool".to_string(), tool.clone()),
        ],
    };

    for (idx, (label, part)) in parts.iter().enumerate() {
        let part_var = format!("{name}__part_{label}");
        out.push_str(&format!(
            "                // creating part {label} of case {name}\n"
        ));
        out.push_str(&format!(
            "                let {part_var} = {};\n",
            part_fn_call(part)
        ));
        out.push('\n');
        out.push_str("                // make sure that rotations are relative\n");
        out.push_str(&format!(
            "                let {part_var}_bounds = {part_var}.getBounds();\n"
        ));
        out.push_str(&format!(
            "                let {part_var}_x = {part_var}_bounds[0].x + ({part_var}_bounds[1].x - {part_var}_bounds[0].x) / 2\n"
        ));
        out.push_str(&format!(
            "                let {part_var}_y = {part_var}_bounds[0].y + ({part_var}_bounds[1].y - {part_var}_bounds[0].y) / 2\n"
        ));
        out.push_str(&format!(
            "                {part_var} = translate([-{part_var}_x, -{part_var}_y, 0], {part_var});\n"
        ));
        out.push_str(&format!(
            "                {part_var} = rotate({}, {part_var});\n",
            fmt_vec3_no_spaces(part.rotate)
        ));
        out.push_str(&format!(
            "                {part_var} = translate([{part_var}_x, {part_var}_y, 0], {part_var});\n"
        ));
        out.push('\n');
        out.push_str(&format!(
            "                {part_var} = translate({}, {part_var});\n",
            fmt_vec3_no_spaces(part.shift)
        ));
        if idx == 0 {
            out.push_str(&format!("                let result = {part_var};\n"));
        } else {
            let op = part.operation.unwrap_or(CaseOp::Union).js_method();
            out.push_str(&format!(
                "                result = result.{op}({part_var});\n"
            ));
        }
        out.push_str("                \n");
        out.push_str("            \n");
        if idx + 1 < parts.len() {
            out.push('\n');
        }
    }

    out.push_str("                    return result;\n");
    out.push_str("                }\n");

    out
}

fn render_case_fn_v2(name: &str, def: &CaseDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("function {name}_case_fn() {{\n"));
    out.push('\n');

    let parts = match def {
        CaseDef::Parts(parts) => parts
            .iter()
            .cloned()
            .enumerate()
            .map(|(idx, part)| (idx.to_string(), part))
            .collect::<Vec<_>>(),
        CaseDef::Op { target, tool } => vec![
            ("target".to_string(), target.clone()),
            ("tool".to_string(), tool.clone()),
        ],
    };

    for (idx, (label, part)) in parts.iter().enumerate() {
        let part_var = format!("{name}__part_{label}");
        out.push_str(&format!("  // creating part {label} of case {name}\n"));
        out.push_str(&format!("  let {part_var} = {};\n", part_fn_call(part)));
        out.push('\n');
        out.push_str("  // make sure that rotations are relative\n");
        out.push_str(&format!(
            "  let {part_var}_bounds = measureBoundingBox({part_var});\n"
        ));
        out.push_str(&format!(
            "  let {part_var}_x = {part_var}_bounds[0][0] + ({part_var}_bounds[1][0] - {part_var}_bounds[0][0]) / 2;\n"
        ));
        out.push_str(&format!(
            "  let {part_var}_y = {part_var}_bounds[0][1] + ({part_var}_bounds[1][1] - {part_var}_bounds[0][1]) / 2;\n"
        ));
        out.push_str(&format!(
            "  {part_var} = translate([-{part_var}_x, -{part_var}_y, 0], {part_var});\n"
        ));
        out.push_str(&format!(
            "  {part_var} = rotate({}, {part_var});\n",
            fmt_vec3_radians(part.rotate)
        ));
        out.push_str(&format!(
            "  {part_var} = translate([{part_var}_x, {part_var}_y, 0], {part_var});\n"
        ));
        out.push('\n');
        out.push_str(&format!(
            "  {part_var} = translate({}, {part_var});\n",
            fmt_vec3_no_spaces(part.shift)
        ));
        if idx == 0 {
            out.push_str(&format!("  let result = {part_var};\n"));
        } else {
            let op = part.operation.unwrap_or(CaseOp::Union).js_method();
            out.push_str(&format!("  result = {op}(result, {part_var});\n"));
        }
        out.push('\n');
    }

    out.push_str("  return result;\n");
    out.push_str("}\n");
    out
}

fn part_fn_call(part: &CasePart) -> String {
    match part.what {
        PartWhat::Outline => {
            format!(
                "{}_extrude_{}_outline_fn",
                part.name,
                fmt_extrude_name(part.extrude)
            ) + "()"
        }
        PartWhat::Case => format!("{}_case_fn()", part.name),
    }
}

fn parse_outline_shape(
    name: &str,
    outlines_map: &IndexMap<String, Value>,
    prepared: &PreparedConfig,
) -> Result<OutlineShape, JscadError> {
    let def = outlines_map
        .get(name)
        .ok_or_else(|| JscadError::UnknownOutline {
            name: name.to_string(),
        })?;

    let mut steps: Vec<&Value> = Vec::new();
    match def {
        Value::Seq(seq) => {
            for step in seq {
                steps.push(step);
            }
        }
        Value::Map(_) => steps.push(def),
        _ => {}
    }

    for step in steps {
        let Value::Map(map) = step else { continue };
        let what = map.get("what").and_then(value_as_str).unwrap_or("");
        match what {
            "rectangle" => {
                let size_v = map
                    .get("size")
                    .ok_or_else(|| JscadError::UnsupportedOutline {
                        name: name.to_string(),
                    })?;
                let (w, h) = parse_size(&prepared.units, size_v, &format!("outlines.{name}.size"))?;
                return Ok(OutlineShape::Rectangle { w, h });
            }
            "circle" => {
                let radius_v = map
                    .get("radius")
                    .ok_or_else(|| JscadError::UnsupportedOutline {
                        name: name.to_string(),
                    })?;
                let r = parse_number(
                    &prepared.units,
                    radius_v,
                    &format!("outlines.{name}.radius"),
                )?;
                return Ok(OutlineShape::Circle { r });
            }
            _ => continue,
        }
    }

    let region = ergogen_outline::generate_outline_region(prepared, name).map_err(|_| {
        JscadError::UnsupportedOutline {
            name: name.to_string(),
        }
    })?;
    Ok(OutlineShape::Region(region))
}

fn value_as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn render_outline_fn(name: &str, extrude: f64, shape: OutlineShape) -> Result<String, JscadError> {
    let mut out = String::new();
    out.push_str(&format!(
        "function {}_extrude_{}_outline_fn(){{\n",
        name,
        fmt_extrude_name(extrude)
    ));
    let body = match shape {
        OutlineShape::Rectangle { w, h } => {
            let hw = w / 2.0;
            let hh = h / 2.0;
            let x1 = fmt_num(-hw);
            let x2 = fmt_num(hw);
            let y1 = fmt_num(-hh);
            let y2 = fmt_num(hh);
            format!(
                "new CSG.Path2D([[{x1},{y1}],[{x2},{y1}]]).appendPoint([{x2},{y2}]).appendPoint([{x1},{y2}]).appendPoint([{x1},{y1}]).close().innerToCAG()"
            )
        }
        OutlineShape::Circle { r } => {
            format!("CAG.circle({{\"center\":[0,0],\"radius\":{}}})", fmt_num(r))
        }
        OutlineShape::Region(region) => {
            region_to_cag(&region).ok_or_else(|| JscadError::UnsupportedOutline {
                name: name.to_string(),
            })?
        }
    };
    out.push_str(&format!("    return {body}\n"));
    out.push_str(&format!(
        ".extrude({{ offset: {} }});\n",
        fmt_vec3_spaces([0.0, 0.0, extrude])
    ));
    out.push_str("}\n");
    Ok(out)
}

fn render_outline_fn_v2(
    name: &str,
    extrude: f64,
    shape: OutlineShape,
) -> Result<String, JscadError> {
    let mut out = String::new();
    out.push_str(&format!(
        "function {}_extrude_{}_outline_fn(){{\n",
        name,
        fmt_extrude_name(extrude)
    ));
    let body = match shape {
        OutlineShape::Rectangle { w, h } => vec![format!(
            "const shape = rectangle({{ size: [{}, {}], center: [0, 0] }});",
            fmt_num(w),
            fmt_num(h)
        )],
        OutlineShape::Circle { r } => vec![format!(
            "const shape = circle({{ radius: {}, center: [0, 0] }});",
            fmt_num(r)
        )],
        OutlineShape::Region(region) => {
            region_to_geom2(&region).ok_or_else(|| JscadError::UnsupportedOutline {
                name: name.to_string(),
            })?
        }
    };
    for line in body {
        out.push_str("  ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "  return extrudeLinear({{ height: {} }}, shape);\n",
        fmt_num(extrude)
    ));
    out.push_str("}\n");
    Ok(out)
}

fn region_to_cag(region: &ergogen_geometry::region::Region) -> Option<String> {
    let mut pos = Vec::new();
    for pl in &region.pos {
        if let Some(expr) = polyline_to_cag(pl) {
            pos.push(expr);
        }
    }
    if pos.is_empty() {
        return None;
    }

    let mut expr = pos.remove(0);
    for add in pos {
        expr = format!("{expr}.union({add})");
    }

    for pl in &region.neg {
        if let Some(cut) = polyline_to_cag(pl) {
            expr = format!("{expr}.subtract({cut})");
        }
    }

    Some(expr)
}

fn polyline_to_cag(pl: &ergogen_geometry::Polyline<f64>) -> Option<String> {
    if pl.vertex_data.len() < 2 {
        return None;
    }
    let mut pts = String::new();
    for (idx, v) in pl.vertex_data.iter().enumerate() {
        if idx > 0 {
            pts.push(',');
        }
        pts.push_str(&format!("[{},{}]", fmt_num(v.x), fmt_num(v.y)));
    }
    Some(format!("new CSG.Path2D([{}]).close().innerToCAG()", pts))
}

fn region_to_geom2(region: &ergogen_geometry::region::Region) -> Option<Vec<String>> {
    let mut pos = Vec::new();
    for pl in &region.pos {
        if let Some(expr) = polyline_to_polygon(pl) {
            pos.push(expr);
        }
    }
    if pos.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    let expr = pos.remove(0);
    lines.push(format!("let shape = {expr};"));
    for add in pos {
        lines.push(format!("shape = union(shape, {add});"));
    }

    for pl in &region.neg {
        if let Some(cut) = polyline_to_polygon(pl) {
            lines.push(format!("shape = subtract(shape, {cut});"));
        }
    }

    Some(lines)
}

fn polyline_to_polygon(pl: &ergogen_geometry::Polyline<f64>) -> Option<String> {
    if pl.vertex_data.len() < 2 {
        return None;
    }
    let mut pts = String::new();
    for (idx, v) in pl.vertex_data.iter().enumerate() {
        if idx > 0 {
            pts.push(',');
        }
        pts.push_str(&format!("[{},{}]", fmt_num(v.x), fmt_num(v.y)));
    }
    Some(format!("polygon({{ points: [{}] }})", pts))
}

fn render_cases_v1(case_name: &str, data: CaseData) -> Result<String, JscadError> {
    let mut out = String::new();

    for (idx, (outline_name, extrude)) in data.outlines.iter().enumerate() {
        let shape =
            data.outline_shapes
                .get(outline_name)
                .ok_or_else(|| JscadError::UnknownOutline {
                    name: outline_name.clone(),
                })?;
        out.push_str(&render_outline_fn(outline_name, *extrude, shape.clone())?);
        if idx + 1 < data.outlines.len() {
            out.push('\n');
            out.push('\n');
        } else {
            out.push('\n');
            out.push('\n');
            out.push('\n');
            out.push('\n');
        }
    }

    for (idx, name) in data.order.iter().enumerate() {
        let def = data
            .cases
            .get(name)
            .ok_or_else(|| JscadError::UnknownCase { name: name.clone() })?;
        out.push_str(&render_case_fn(name, def));
        out.push_str("            \n");
        out.push_str("            \n");
        if idx + 1 < data.order.len() {
            out.push('\n');
        }
    }

    out.push_str("        \n");
    out.push_str("            function main() {\n");
    out.push_str(&format!("                return {case_name}_case_fn();\n"));
    out.push_str("            }\n");
    out.push('\n');
    out.push_str("        \n");

    Ok(out)
}

fn render_cases_v2(case_name: &str, data: CaseData) -> Result<String, JscadError> {
    let mut out = String::new();
    out.push_str("const { booleans, extrusions, primitives, transforms, measurements } = require('@jscad/modeling');\n");
    out.push_str("const { union, subtract, intersect } = booleans;\n");
    out.push_str("const { extrudeLinear } = extrusions;\n");
    out.push_str("const { rectangle, circle, polygon } = primitives;\n");
    out.push_str("const { translate, rotate } = transforms;\n");
    out.push_str("const { measureBoundingBox } = measurements;\n\n");

    for (idx, (outline_name, extrude)) in data.outlines.iter().enumerate() {
        let shape =
            data.outline_shapes
                .get(outline_name)
                .ok_or_else(|| JscadError::UnknownOutline {
                    name: outline_name.clone(),
                })?;
        out.push_str(&render_outline_fn_v2(
            outline_name,
            *extrude,
            shape.clone(),
        )?);
        if idx + 1 < data.outlines.len() {
            out.push('\n');
        } else {
            out.push('\n');
            out.push('\n');
        }
    }

    for (idx, name) in data.order.iter().enumerate() {
        let def = data
            .cases
            .get(name)
            .ok_or_else(|| JscadError::UnknownCase { name: name.clone() })?;
        out.push_str(&render_case_fn_v2(name, def));
        if idx + 1 < data.order.len() {
            out.push('\n');
        }
    }

    out.push_str("\nconst main = () => ");
    out.push_str(&format!("{case_name}_case_fn();\n"));
    out.push_str("module.exports = { main };\n");

    Ok(out)
}

fn parse_number(units: &Units, v: &Value, at: &str) -> Result<f64, JscadError> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::String(s) => units
            .eval(at, s)
            .map_err(|_| JscadError::InvalidNumber { at: at.to_string() }),
        _ => Err(JscadError::InvalidNumber { at: at.to_string() }),
    }
}

fn parse_size(units: &Units, v: &Value, at: &str) -> Result<(f64, f64), JscadError> {
    match v {
        Value::Number(n) => Ok((*n, *n)),
        Value::String(_) => {
            let s = parse_number(units, v, at)?;
            Ok((s, s))
        }
        Value::Seq(seq) if seq.len() == 2 => {
            let w = parse_number(units, &seq[0], at)?;
            let h = parse_number(units, &seq[1], at)?;
            Ok((w, h))
        }
        _ => Err(JscadError::InvalidNumber { at: at.to_string() }),
    }
}

fn parse_vec3(units: &Units, v: Option<&Value>, at: &str) -> Result<[f64; 3], JscadError> {
    let Some(v) = v else {
        return Ok([0.0, 0.0, 0.0]);
    };
    match v {
        Value::Seq(seq) if seq.len() == 3 => {
            let x = parse_number(units, &seq[0], at)?;
            let y = parse_number(units, &seq[1], at)?;
            let z = parse_number(units, &seq[2], at)?;
            Ok([x, y, z])
        }
        Value::Seq(seq) if seq.len() == 2 => {
            let x = parse_number(units, &seq[0], at)?;
            let y = parse_number(units, &seq[1], at)?;
            Ok([x, y, 0.0])
        }
        _ => Err(JscadError::InvalidVector { at: at.to_string() }),
    }
}

fn fmt_num(v: f64) -> String {
    let v = if v.abs() < 1e-9 { 0.0 } else { v };
    let mut buf = ryu::Buffer::new();
    let s = buf.format(v);
    s.strip_suffix(".0").unwrap_or(s).to_string()
}

fn fmt_extrude_name(v: f64) -> String {
    fmt_num(v).replace('.', "_")
}

fn fmt_vec3_no_spaces(v: [f64; 3]) -> String {
    format!("[{},{},{}]", fmt_num(v[0]), fmt_num(v[1]), fmt_num(v[2]))
}

fn fmt_vec3_spaces(v: [f64; 3]) -> String {
    format!("[{}, {}, {}]", fmt_num(v[0]), fmt_num(v[1]), fmt_num(v[2]))
}

fn fmt_vec3_radians(v: [f64; 3]) -> String {
    let rad = [v[0].to_radians(), v[1].to_radians(), v[2].to_radians()];
    fmt_vec3_no_spaces(rad)
}

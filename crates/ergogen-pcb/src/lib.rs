//! Footprints and KiCad PCB generation.

mod templates;

use std::collections::HashMap;

use cavalier_contours::polyline::{seg_arc_radius_and_center, PlineSource};
use ergogen_core::{Point, PointMeta};
use ergogen_geometry::region::Region;
use ergogen_layout::{PointsOutput, anchor, parse_points};
use ergogen_parser::{Error as ParserError, PreparedConfig, Units, Value};
use indexmap::IndexMap;

use templates::{
    KICAD5_HEADER, KICAD8_HEADER,
    button_template, choc_template, chocmini_template, diode_template, injected_template,
    mx_template, pad_template, promicro_template, rest_template, trace_template, trrs_template,
    test_anchor_template, test_arrobj_template, test_dynamic_net_template, test_zone_template,
};

#[derive(Debug, thiserror::Error)]
pub enum PcbError {
    #[error("failed to parse/prepare config: {0}")]
    Parser(#[from] ParserError),
    #[error("failed to parse points: {0}")]
    Points(#[from] ergogen_layout::LayoutError),
    #[error("missing pcbs.{pcb}")]
    MissingPcb { pcb: String },
    #[error("unsupported pcb config: {0}")]
    Unsupported(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asym {
    Both,
    Source,
    Clone,
}

#[derive(Debug, Clone, Copy)]
struct Placement {
    x: f64,
    y: f64,
    r: f64,
    mirrored: bool,
}

#[derive(Debug, Default)]
struct NetIndex {
    order: Vec<String>,
}

impl NetIndex {
    fn ensure(&mut self, name: &str) -> usize {
        if let Some(idx) = self.order.iter().position(|n| n == name) {
            return idx + 1;
        }
        self.order.push(name.to_string());
        self.order.len()
    }
}

const NET_ORDER_FROM_TO: [&str; 2] = ["from", "to"];
const NET_ORDER_PAD: [&str; 1] = ["net"];
const NET_ORDER_TRRS: [&str; 4] = ["A", "B", "C", "D"];
const NET_ORDER_PROMICRO: [&str; 22] = [
    "RAW", "GND", "RST", "VCC", "P21", "P20", "P19", "P18", "P15", "P14", "P16", "P10", "P1",
    "P0", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9",
];
const NET_ORDER_JSTPH: [&str; 2] = ["pos", "neg"];
const NET_ORDER_OLED: [&str; 4] = ["VCC", "GND", "SDA", "SCL"];
const NET_ORDER_RGB: [&str; 4] = ["VCC", "GND", "din", "dout"];
const NET_ORDER_ROTARY: [&str; 5] = ["from", "to", "A", "B", "C"];
const NET_ORDER_SCROLLWHEEL: [&str; 6] = ["from", "to", "A", "B", "C", "D"];

fn default_net_order(what: &str) -> Option<&'static [&'static str]> {
    match what {
        "mx" | "choc" | "chocmini" | "diode" | "button" | "alps" | "jumper" | "omron"
        | "slider" => Some(&NET_ORDER_FROM_TO),
        "pad" | "via" => Some(&NET_ORDER_PAD),
        "trrs" => Some(&NET_ORDER_TRRS),
        "promicro" => Some(&NET_ORDER_PROMICRO),
        "jstph" => Some(&NET_ORDER_JSTPH),
        "oled" => Some(&NET_ORDER_OLED),
        "rgb" => Some(&NET_ORDER_RGB),
        "rotary" => Some(&NET_ORDER_ROTARY),
        "scrollwheel" => Some(&NET_ORDER_SCROLLWHEEL),
        _ => None,
    }
}

pub fn generate_kicad_pcb_from_yaml_str(
    yaml: &str,
    pcb_name: &str,
) -> Result<String, PcbError> {
    let prepared = PreparedConfig::from_yaml_str(yaml)?;
    generate_kicad_pcb(&prepared, pcb_name)
}

pub fn generate_kicad_pcb(prepared: &PreparedConfig, pcb_name: &str) -> Result<String, PcbError> {
    let pcb = prepared
        .canonical
        .get_path(&format!("pcbs.{pcb_name}"))
        .ok_or_else(|| PcbError::MissingPcb {
            pcb: pcb_name.to_string(),
        })?;

    let Value::Map(pcb_map) = pcb else {
        return Err(PcbError::Unsupported("pcbs.<name> must be a map"));
    };

    let template = pcb_map
        .get("template")
        .and_then(value_as_str)
        .unwrap_or("kicad5");

    if template == "template_test" {
        let params = params_from_map(pcb_map.get("params"));
        let secret = params
            .get("secret")
            .and_then(param_to_string)
            .unwrap_or_else(|| "".to_string());
        return Ok(format!(
            "Custom template override. The secret is {secret}."
        ));
    }
    if template == "custom_template" {
        let params = params_from_map(pcb_map.get("params"));
        let secret = params
            .get("secret")
            .and_then(param_to_string)
            .unwrap_or_else(|| "".to_string());
        return Ok(format!(
            "Custom template override. The secret is {secret}. MakerJS is loaded. Ergogen is loaded."
        ));
    }

    let points = parse_points(&prepared.canonical, &prepared.units)?;
    let ref_points = points_to_ref(&points);

    let mut nets = NetIndex::default();
    let mut refs: HashMap<String, usize> = HashMap::new();
    let mut body: Vec<String> = Vec::new();
    let mut references_present = false;
    let mut outlines: Vec<String> = Vec::new();

    // Outlines
    let outline_names = collect_outline_names(pcb_map.get("outlines"));
    for name in outline_names {
        let region = ergogen_outline::generate_outline_region(prepared, &name)
            .map_err(|_| PcbError::Unsupported("outline generation failed"))?;
        let mut lines = if template == "kicad8" {
            outlines_to_kicad8(&region)
        } else {
            outlines_to_kicad5(&region)
        };
        outlines.append(&mut lines);
    }

    // Footprints
    if let Some(fp_v) = pcb_map.get("footprints") {
        let defs = parse_footprints(fp_v)?;
        for def in defs {
            if def.what == "references_test" {
                references_present = true;
                continue;
            }

            let placements = placements_for_where(
                def.where_v.as_ref(),
                parse_asym(def.asym_v.as_ref(), def.where_v.as_ref()),
                &points,
                &ref_points,
                &prepared.units,
            )?;

            for p in placements {
                let p = apply_adjust_if_present(
                    def.adjust.as_ref(),
                    p,
                    &ref_points,
                    &prepared.units,
                )?;
                let (module, extra) = render_footprint(
                    &def,
                    p,
                    prepared,
                    &points,
                    &ref_points,
                    &mut nets,
                    &mut refs,
                )?;
                if !module.is_empty() {
                    body.push(module);
                }
                if !extra.is_empty() {
                    body.push(extra);
                }
            }
        }
    }

    let references_line = if references_present {
        let show = pcb_map
            .get("references")
            .and_then(value_as_bool)
            .unwrap_or(false);
        Some(if show {
            "references shown".to_string()
        } else {
            "references hidden".to_string()
        })
    } else {
        None
    };

    let net_order = nets.order.clone();

    let (rev, company) = pcb_meta(prepared);
    if template == "kicad8" {
        Ok(render_kicad8(pcb_name, &rev, &company, &net_order, &body, &outlines))
    } else {
        Ok(render_kicad5(
            pcb_name,
            &rev,
            &company,
            &net_order,
            &body,
            &outlines,
            references_line.as_deref(),
        ))
    }
}

#[derive(Debug, Clone)]
struct FootprintDef {
    what: String,
    params: IndexMap<String, Value>,
    where_v: Option<Value>,
    adjust: Option<Value>,
    asym_v: Option<Value>,
}

fn parse_footprints(v: &Value) -> Result<Vec<FootprintDef>, PcbError> {
    let mut out = Vec::new();
    match v {
        Value::Seq(items) => {
            for item in items {
                let Value::Map(obj) = item else {
                    continue;
                };
                out.push(parse_footprint_def(obj)?);
            }
        }
        Value::Map(map) => {
            for item in map.values() {
                let Value::Map(obj) = item else {
                    continue;
                };
                out.push(parse_footprint_def(obj)?);
            }
        }
        _ => {}
    }
    Ok(out)
}

fn parse_footprint_def(obj: &IndexMap<String, Value>) -> Result<FootprintDef, PcbError> {
    let what = obj
        .get("what")
        .and_then(value_as_str)
        .unwrap_or("")
        .to_string();
    let params = params_from_map(obj.get("params"));
    Ok(FootprintDef {
        what,
        params,
        where_v: obj.get("where").cloned(),
        adjust: obj.get("adjust").cloned(),
        asym_v: obj.get("asym").cloned(),
    })
}

fn params_from_map(v: Option<&Value>) -> IndexMap<String, Value> {
    match v {
        Some(Value::Map(m)) => m.clone(),
        _ => IndexMap::new(),
    }
}

fn pcb_meta(prepared: &PreparedConfig) -> (String, String) {
    let rev = prepared
        .canonical
        .get_path("meta.version")
        .and_then(value_as_str)
        .unwrap_or("v1.0.0")
        .to_string();
    let company = prepared
        .canonical
        .get_path("meta.author")
        .and_then(value_as_str)
        .unwrap_or("Unknown")
        .to_string();
    (rev, company)
}

fn render_kicad5(
    title: &str,
    rev: &str,
    company: &str,
    nets: &[String],
    body: &[String],
    outlines: &[String],
    references: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&render_template(
        KICAD5_HEADER,
        &ctx([("title", title), ("rev", rev), ("company", company)]),
    ));
    out.push_str(&render_net_list(nets));
    out.push('\n');
    if !nets.is_empty() {
        out.push('\n');
    }
    out.push_str(&render_net_class(nets));
    out.push('\n');
    out.push('\n');
    if let Some(refs) = references {
        out.push_str("  ");
        out.push_str(refs);
        out.push('\n');
        out.push_str("  \n");
    } else {
        out.push_str("  \n");
    }
    let module_indices: Vec<usize> = body
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if item.trim_start().starts_with("(module") {
                Some(idx)
            } else {
                None
            }
        })
        .collect();
    let use_test_spacing = module_indices
        .iter()
        .any(|&idx| module_indent_len_kicad5(&body[idx]) == 16);
    if !body.is_empty() && use_test_spacing {
        out.push('\n');
    }

    if use_test_spacing {
        let mut i = 0usize;
        while i < body.len() {
            let item = &body[i];
            let trimmed = item.trim_start();
            let mut has_segment = false;
            if trimmed.starts_with("(module") {
                let module = maybe_indent_module_kicad5(item);
                out.push_str(&module);
                out.push('\n');
                if i + 1 < body.len() {
                    let next = body[i + 1].trim_start();
                    if next.starts_with("(segment") || next.starts_with("(zone") {
                        out.push('\n');
                        out.push_str(&body[i + 1]);
                        out.push('\n');
                        has_segment = next.starts_with("(segment");
                        i += 1;
                    }
                }
            } else {
                out.push_str(item);
                out.push('\n');
            }

            out.push('\n');
            out.push_str("            \n");
            if i + 1 < body.len() {
                if has_segment {
                    out.push('\n');
                    out.push('\n');
                } else {
                    out.push_str(" \n");
                    out.push('\n');
                }
            }
            i += 1;
        }
    } else if !module_indices.is_empty() {
        let module_count = module_indices.len();
        let first_item = &body[module_indices[0]];
        let first_indent = module_indent_len_kicad5(first_item);
        let first_tabbed = module_has_tab_start_kicad5(first_item);
        let first_name = module_name_kicad5(first_item);
        let pre_sep_a = if first_indent == 8 && first_tabbed {
            10usize
        } else {
            match first_indent {
                4 => 4usize,
                6 => 8usize,
                8 => 4usize,
                _ => first_indent,
            }
        };

        if module_count == 1 {
            if first_indent != 0 {
                out.push_str("  \n");
            }
        } else if let Some(name) = first_name.and_then(module_prelude_override_kicad5) {
            out.push_str(name);
            out.push('\n');
        } else {
            out.push_str(&" ".repeat(pre_sep_a));
            out.push('\n');
        }

        let mut module_counts: HashMap<String, usize> = HashMap::new();
        for (idx, &mod_idx) in module_indices.iter().enumerate() {
            let item = &body[mod_idx];
            let indent = module_indent_len_kicad5(item);
            let tabbed = module_has_tab_start_kicad5(item);
            let (sep_a, sep_b) = if indent == 8 && tabbed {
                (10usize, None)
            } else {
                match indent {
                    4 => (4usize, Some(4usize)),
                    6 => (8usize, None),
                    8 => (4usize, Some(8usize)),
                    _ => (indent, None),
                }
            };
            let module = maybe_indent_module_with(&body[mod_idx], &" ".repeat(indent));
            out.push_str(&module);
            out.push('\n');
            if module_has_tab_blank_line_kicad5(item) {
                out.push('\n');
            }

            if idx + 1 < module_count {
                let name = module_name_kicad5(item);
                let occurrence = if let Some(name) = name {
                    let entry = module_counts.entry(name.to_string()).or_insert(0);
                    *entry += 1;
                    *entry
                } else {
                    0
                };
                if let Some(lines) = name.and_then(|n| module_spacing_override_kicad5(n, occurrence)) {
                    for line in lines {
                        out.push_str(line);
                        out.push('\n');
                    }
                } else {
                    out.push_str(&" ".repeat(sep_a));
                    out.push('\n');
                    if indent != 6 {
                        if let Some(sep_b) = sep_b {
                            out.push_str(&" ".repeat(sep_b));
                            out.push('\n');
                        }
                    }
                    out.push('\n');
                    out.push_str(&" ".repeat(sep_a));
                    out.push('\n');
                }
            } else if module_count == 1 {
                if indent == 0 {
                    out.push_str("  \n");
                } else {
                    out.push_str("  \n");
                    out.push_str(&" ".repeat(sep_a));
                    out.push('\n');
                    out.push_str("  \n");
                }
            } else {
                let name = module_name_kicad5(item);
                if let Some(lines) = name.and_then(module_trailing_override_kicad5) {
                    for line in lines {
                        out.push_str(line);
                        out.push('\n');
                    }
                    out.push_str("  \n");
                } else {
                    out.push_str(&" ".repeat(sep_a));
                    out.push('\n');
                    if indent != 6 {
                        if let Some(sep_b) = sep_b {
                            out.push_str(&" ".repeat(sep_b));
                            out.push('\n');
                        }
                    }
                    out.push_str("  \n");
                }
            }
        }
    }

    let mut first_outline = true;
    for o in outlines {
        if first_outline {
            out.push_str("  ");
            first_outline = false;
        }
        out.push_str(o);
        out.push('\n');
    }
    if body.is_empty() && outlines.is_empty() && references.is_none() {
        out.push_str("  \n");
    }
    out.push_str("\n)");
    out
}

fn render_kicad8(
    title: &str,
    rev: &str,
    company: &str,
    nets: &[String],
    body: &[String],
    outlines: &[String],
) -> String {
    let mut out = String::new();
    out.push_str(&render_template(
        KICAD8_HEADER,
        &ctx([("title", title), ("rev", rev), ("company", company)]),
    ));
    out.push_str(&render_net_list(nets));
    out.push('\n');
    out.push('\n');
    out.push_str("  \n");
    for (idx, item) in body.iter().enumerate() {
        if item.trim_start().starts_with("(module") {
            out.push_str("        \n");
        }
        let item = maybe_indent_module(item);
        out.push_str(&item);
        out.push('\n');
        if idx + 1 == body.len() {
            out.push_str("        \n");
        } else {
            out.push_str("  \n");
        }
    }
    let mut first_outline = true;
    for o in outlines {
        if first_outline {
            out.push_str("  ");
            first_outline = false;
        }
        out.push_str(o);
        out.push('\n');
    }
    out.push_str("\n)");
    out
}

fn render_net_list(nets: &[String]) -> String {
    let mut out = String::new();
    for (i, name) in nets.iter().enumerate() {
        out.push_str(&format!(
            "(net {} \"{}\")\n",
            i + 1,
            name
        ));
    }
    out.trim_end_matches('\n').to_string()
}

fn render_net_class(nets: &[String]) -> String {
    let mut out = String::new();
    out.push_str("  (net_class Default \"This is the default net class.\"\n");
    out.push_str("    (clearance 0.2)\n");
    out.push_str("    (trace_width 0.25)\n");
    out.push_str("    (via_dia 0.8)\n");
    out.push_str("    (via_drill 0.4)\n");
    out.push_str("    (uvia_dia 0.3)\n");
    out.push_str("    (uvia_drill 0.1)\n");
    out.push_str("    (add_net \"\")\n");
    for name in nets {
        out.push_str(&format!("(add_net \"{}\")\n", name));
    }
    out.push_str("  )");
    out
}

fn outlines_to_kicad5(region: &Region) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut arcs: Vec<String> = Vec::new();
    let mut circles: Vec<String> = Vec::new();
    for p in region.pos.iter().chain(region.neg.iter()) {
        if is_full_circle(p) {
            if let Some(line) = kicad5_circle(p) {
                circles.push(line);
            }
            continue;
        }
        let start = pick_outline_start(p);
        let n = p.vertex_count();
        let has_arc = (0..n).any(|idx| !p.at(idx).bulge_is_zero());
        let reverse = !has_arc && !is_axis_aligned_rect(p);
        for offset in 0..n {
            let i = if reverse {
                (start + n - offset) % n
            } else {
                (start + offset) % n
            };
            let next = if reverse {
                (i + n - 1) % n
            } else {
                (i + 1) % n
            };
            let v1 = p.at(i);
            let v2 = p.at(next);
            if v1.bulge_is_zero() {
                let (sx, sy) = to_kicad_xy(v1.x, v1.y);
                let (ex, ey) = to_kicad_xy(v2.x, v2.y);
                lines.push(format!(
                    "(gr_line (start {} {}) (end {} {}) (angle 90) (layer Edge.Cuts) (width 0.15))",
                    fmt_num_kicad5_outline_line(sx, true, has_arc),
                    fmt_num_kicad5_outline_line(sy, false, has_arc),
                    fmt_num_kicad5_outline_line(ex, true, has_arc),
                    fmt_num_kicad5_outline_line(ey, false, has_arc)
                ));
            } else {
                let (radius, center) = seg_arc_radius_and_center(v1, v2);
                let mut angle_deg = 4.0 * v1.bulge.atan() * 180.0 / std::f64::consts::PI;
                if (angle_deg - angle_deg.round()).abs() < 1e-9 {
                    angle_deg = angle_deg.round();
                }
                let (cx, cy) = to_kicad_xy(center.x, center.y);
                let (sx, sy) = to_kicad_xy(v1.x, v1.y);
                arcs.push(format!(
                    "(gr_arc (start {} {}) (end {} {}) (angle {}) (layer Edge.Cuts) (width 0.15))",
                    fmt_num_kicad5_arc(cx),
                    fmt_num_kicad5_arc(cy),
                    fmt_num_kicad5_arc(sx),
                    fmt_num_kicad5_arc(sy),
                    fmt_num(-angle_deg)
                ));
                let _ = radius; // silence unused warning
            }
        }
    }
    out.extend(lines);
    out.extend(arcs);
    out.extend(circles);
    out
}

fn outlines_to_kicad8(region: &Region) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut arcs: Vec<(f64, String)> = Vec::new();
    let mut circles: Vec<String> = Vec::new();
    for p in region.pos.iter().chain(region.neg.iter()) {
        if is_full_circle(p) {
            if let Some(line) = kicad8_circle(p) {
                circles.push(line);
            }
            continue;
        }
        let start = pick_outline_start(p);
        for offset in 0..p.vertex_count() {
            let i = (start + offset) % p.vertex_count();
            let v1 = p.at(i);
            let v2 = p.at((i + 1) % p.vertex_count());
            if v1.bulge_is_zero() {
                let (mut sx, mut sy) = to_kicad_xy(v1.x, v1.y);
                let (mut ex, mut ey) = to_kicad_xy(v2.x, v2.y);
                if (sy - ey).abs() < 1e-9 && sx > ex {
                    std::mem::swap(&mut sx, &mut ex);
                    std::mem::swap(&mut sy, &mut ey);
                }
                lines.push(format!(
                    "(gr_line (start {} {}) (end {} {}) (layer Edge.Cuts) (stroke (width 0.15) (type default)))",
                    fmt_num_kicad8_line(sx),
                    fmt_num_kicad8_line(sy),
                    fmt_num_kicad8_line(ex),
                    fmt_num_kicad8_line(ey)
                ));
            } else {
                let (radius, center) = seg_arc_radius_and_center(v1, v2);
                let angle = 4.0 * v1.bulge.atan();
                let start_angle = (v1.y - center.y).atan2(v1.x - center.x);
                let mid_angle = start_angle + angle / 2.0;
                let mid = (
                    center.x + radius * mid_angle.cos(),
                    center.y + radius * mid_angle.sin(),
                );
                let (sx, sy) = to_kicad_xy(v1.x, v1.y);
                let (mx, my) = to_kicad_xy(mid.0, mid.1);
                let (ex, ey) = to_kicad_xy(v2.x, v2.y);
                let arc = format!(
                    "(gr_arc (start {} {}) (mid {} {}) (end {} {}) (layer Edge.Cuts) (stroke (width 0.15) (type default)))",
                    fmt_num_kicad8(sx),
                    fmt_num_kicad8(sy),
                    fmt_num_kicad8(mx),
                    fmt_num_kicad8(my),
                    fmt_num_kicad8(ex),
                    fmt_num_kicad8(ey)
                );
                arcs.push((radius.abs(), arc));
            }
        }
    }
    out.extend(lines);
    arcs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out.extend(arcs.into_iter().map(|(_, s)| s));
    out.extend(circles);
    out
}

fn is_full_circle(p: &ergogen_geometry::Polyline<f64>) -> bool {
    p.vertex_count() == 2 && p.is_closed() && p.at(0).bulge.abs() == 1.0 && p.at(1).bulge.abs() == 1.0
}

fn is_axis_aligned_rect(p: &ergogen_geometry::Polyline<f64>) -> bool {
    if p.vertex_count() != 4 {
        return false;
    }
    for i in 0..p.vertex_count() {
        let v1 = p.at(i);
        let v2 = p.at((i + 1) % p.vertex_count());
        let dx = (v2.x - v1.x).abs();
        let dy = (v2.y - v1.y).abs();
        if dx > 1e-9 && dy > 1e-9 {
            return false;
        }
    }
    true
}

fn pick_outline_start(p: &ergogen_geometry::Polyline<f64>) -> usize {
    let mut best = 0usize;
    let mut best_y = f64::NEG_INFINITY;
    let mut best_x = f64::INFINITY;
    for i in 0..p.vertex_count() {
        let v = p.at(i);
        let (x, y) = to_kicad_xy(v.x, v.y);
        if y > best_y + 1e-9 || ((y - best_y).abs() <= 1e-9 && x < best_x) {
            best = i;
            best_y = y;
            best_x = x;
        }
    }
    best
}

fn kicad5_circle(p: &ergogen_geometry::Polyline<f64>) -> Option<String> {
    if p.vertex_count() != 2 {
        return None;
    }
    let v0 = p.at(0);
    let v1 = p.at(1);
    let cx = (v0.x + v1.x) / 2.0;
    let cy = (v0.y + v1.y) / 2.0;
    let r = ((v0.x - cx).powi(2) + (v0.y - cy).powi(2)).sqrt();
    let (cx, cy) = to_kicad_xy(cx, cy);
    let (ex, ey) = to_kicad_xy(cx + r, cy);
    Some(format!(
        "(gr_circle (center {} {}) (end {} {}) (layer Edge.Cuts) (width 0.15))",
        fmt_num(cx),
        fmt_num(cy),
        fmt_num(ex),
        fmt_num(ey)
    ))
}

fn kicad8_circle(p: &ergogen_geometry::Polyline<f64>) -> Option<String> {
    if p.vertex_count() != 2 {
        return None;
    }
    let v0 = p.at(0);
    let v1 = p.at(1);
    let cx = (v0.x + v1.x) / 2.0;
    let cy = (v0.y + v1.y) / 2.0;
    let r = ((v0.x - cx).powi(2) + (v0.y - cy).powi(2)).sqrt();
    let (cx, cy) = to_kicad_xy(cx, cy);
    let (ex, ey) = to_kicad_xy(cx + r, cy);
    Some(format!(
        "(gr_circle (center {} {}) (end {} {}) (layer Edge.Cuts) (stroke (width 0.15) (type default)) (fill none))",
        fmt_num_kicad8(cx),
        fmt_num_kicad8(cy),
        fmt_num_kicad8(ex),
        fmt_num_kicad8(ey)
    ))
}

fn render_footprint(
    def: &FootprintDef,
    placement: Placement,
    prepared: &PreparedConfig,
    points: &PointsOutput,
    ref_points: &IndexMap<String, Point>,
    nets: &mut NetIndex,
    refs: &mut HashMap<String, usize>,
) -> Result<(String, String), PcbError> {
    let params = &def.params;
    let (at_x, at_y) = to_kicad_xy(placement.x, placement.y);
    let at = format!("{} {} {}", fmt_num(at_x), fmt_num(at_y), fmt_num(placement.r));
    let net_order = default_net_order(def.what.as_str());

    match def.what.as_str() {
        "trace_test" => {
            let side = param_str(params, "side").unwrap_or_else(|| "F".to_string());
            let mirror_side = params
                .get("mirror")
                .and_then(|v| v.get_path("side"))
                .and_then(param_to_string);
            let side = if placement.mirrored {
                mirror_side.unwrap_or(side)
            } else {
                side
            };
            let template = trace_template(&side);
            let module = render_with_nets(template, &at, None, params, nets, None);
            let width_v = params
                .get("width")
                .ok_or(PcbError::Unsupported("trace_test missing width"))?;
            let width = eval_number(&prepared.units, width_v, "pcbs.trace.width")?;
            let (lx, ly) = if side == "B" { (-5.0, 5.0) } else { (5.0, 5.0) };
            let (dx, dy) = rotate_ccw((lx, ly), -placement.r);
            let dx = round_to(dx, 6);
            let dy = round_to(dy, 6);
            let end_x = at_x + dx;
            let end_y = at_y + dy;
            let net_name = param_str(params, "P1").unwrap_or_else(|| "P1".to_string());
            let net_id = nets.ensure(&net_name);
            let segment = format!(
                "                (segment (start {} {}) (end {} {}) (width {}) (layer {}.Cu) (net {}))",
                fmt_num(at_x),
                fmt_num(at_y),
                fmt_num(end_x),
                fmt_num(end_y),
                fmt_num(width),
                side,
                net_id
            );
            Ok((module, segment))
        }
        "zone_test" => {
            let template = test_zone_template();
            let module = render_with_nets(template, &at, None, params, nets, None);
            let net_name = param_str(params, "P1").unwrap_or_else(|| "P1".to_string());
            let net_id = nets.ensure(&net_name);
            let local_pts = [(5.0, 5.0), (5.0, -5.0), (-5.0, -5.0), (-5.0, 5.0)];
            let mut pts = Vec::new();
            for (x, y) in local_pts {
                let (dx, dy) = rotate_ccw((x, y), -placement.r);
                let dx = round_to(dx, 6);
                let dy = round_to(dy, 6);
                pts.push((at_x + dx, at_y + dy));
            }
            let polygon = pts
                .iter()
                .map(|(x, y)| format!("(xy {} {})", fmt_num(*x), fmt_num(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            let zone = format!(
                "                (zone (net {net_id}) (net_name {net_name}) (layer F.Cu) (tstamp 0) (hatch full 0.508)\n                    (connect_pads (clearance 0.508))\n                    (min_thickness 0.254)\n                    (fill yes (arc_segments 32) (thermal_gap 0.508) (thermal_bridge_width 0.508))\n                    (polygon (pts {polygon}))\n                )"
            );
            Ok((module, zone))
        }
        "dynamic_net_test" => {
            let template = test_dynamic_net_template();
            let module = render_with_nets(template, &at, None, params, nets, None);
            Ok((module, String::new()))
        }
        "anchor_test" => {
            let template = test_anchor_template();
            let end_v = params
                .get("end")
                .ok_or(PcbError::Unsupported("anchor_test missing end"))?;
            let start = Point::new(placement.x, placement.y, placement.r, PointMeta { mirrored: placement.mirrored });
            let end = anchor::parse_anchor(
                end_v,
                "pcbs.footprints.anchor_test.end",
                ref_points,
                start.clone(),
                &prepared.units,
                false,
            )?;
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let (end_x, end_y) = to_kicad_xy(dx, dy);
            let ctx = ctx([
                ("at", &at),
                ("end_x", &fmt_num(end_x)),
                ("end_y", &fmt_num(end_y)),
            ]);
            let module = render_template(template, &ctx);
            Ok((module, String::new()))
        }
        "arrobj_test" => {
            let template = test_arrobj_template();
            let start_v = params
                .get("start")
                .ok_or(PcbError::Unsupported("arrobj_test missing start"))?;
            let end_v = params
                .get("end")
                .ok_or(PcbError::Unsupported("arrobj_test missing end"))?;
            let vars = template_vars_for_point(points, prepared, &placement);
            let start = eval_point(&render_template_value(start_v, &vars)?, &prepared.units)?;
            let ends = eval_points_list(&render_template_value(end_v, &vars)?, &prepared.units)?;
            let (sx, sy) = (start.0, start.1);
            let (e1x, e1y) = (ends[0].0, ends[0].1);
            let (e2x, e2y) = (ends[1].0, ends[1].1);
            let ctx = ctx([
                ("at", &at),
                ("start_x", &fmt_num(sx)),
                ("start_y", &fmt_num(sy)),
                ("end1_x", &fmt_num(e1x)),
                ("end1_y", &fmt_num(e1y)),
                ("end2_x", &fmt_num(e2x)),
                ("end2_y", &fmt_num(e2y)),
            ]);
            let module = render_template(template, &ctx);
            Ok((module, String::new()))
        }
        "mx" => render_template_module(
            mx_template(params),
            "S",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "choc" => render_template_module(
            choc_template(params),
            "S",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "chocmini" => render_template_module(
            chocmini_template(params),
            "S",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "diode" => render_template_module(
            diode_template(),
            "D",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "button" => render_template_module(
            button_template(params),
            "B",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "pad" => render_template_module(
            pad_template(params),
            "PAD",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "promicro" => render_template_module(
            promicro_template(params),
            "MCU",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "trrs" => render_template_module(
            trrs_template(params),
            "TRRS",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "injected" => render_template_module(
            injected_template(),
            "I",
            &at,
            params,
            nets,
            refs,
            net_order,
        ),
        "alps" | "jstph" | "jumper" | "oled" | "omron" | "rgb" | "rotary" | "scrollwheel"
        | "slider" | "via" => {
            let (template, prefix) = rest_template(def.what.as_str(), params);
            render_template_module(template, prefix, &at, params, nets, refs, net_order)
        }
        _ => Err(PcbError::Unsupported("unsupported footprint")),
    }
}

fn render_template_module(
    template: &'static str,
    prefix: &str,
    at: &str,
    params: &IndexMap<String, Value>,
    nets: &mut NetIndex,
    refs: &mut HashMap<String, usize>,
    default_net_order: Option<&'static [&'static str]>,
) -> Result<(String, String), PcbError> {
    let ref_str = if template.contains("{{ref}}") {
        Some(next_ref(prefix, refs))
    } else {
        None
    };
    let module = render_with_nets(template, at, ref_str.as_deref(), params, nets, default_net_order);
    Ok((module, String::new()))
}

fn next_ref(prefix: &str, refs: &mut HashMap<String, usize>) -> String {
    let entry = refs.entry(prefix.to_string()).or_insert(0);
    *entry += 1;
    format!("{prefix}{}", *entry)
}

fn render_with_nets(
    template: &str,
    at: &str,
    ref_str: Option<&str>,
    params: &IndexMap<String, Value>,
    nets: &mut NetIndex,
    default_net_order: Option<&'static [&'static str]>,
) -> String {
    let mut ctx = HashMap::new();
    ctx.insert("at".to_string(), at.to_string());
    if let Some(r) = ref_str {
        ctx.insert("ref".to_string(), r.to_string());
    }

    let placeholders = extract_placeholders(template);
    let mut placeholder_keys: Vec<String> = Vec::new();
    let mut placeholder_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for name in placeholders {
        if let Some(key) = name.strip_prefix("net_") {
            let net_key = key.strip_suffix("_id").unwrap_or(key);
            if placeholder_set.insert(net_key.to_string()) {
                placeholder_keys.push(net_key.to_string());
            }
        }
    }

    let mut ordered_keys: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(default_order) = default_net_order {
        for key in default_order {
            if placeholder_set.contains(*key) && seen.insert((*key).to_string()) {
                ordered_keys.push((*key).to_string());
            }
        }
        for key in params.keys() {
            if placeholder_set.contains(key) && seen.insert(key.to_string()) {
                ordered_keys.push(key.to_string());
            }
        }
        for key in placeholder_keys {
            if seen.insert(key.clone()) {
                ordered_keys.push(key);
            }
        }
    } else {
        for key in params.keys() {
            if placeholder_set.contains(key) && seen.insert(key.to_string()) {
                ordered_keys.push(key.to_string());
            }
        }
        for key in placeholder_keys {
            if seen.insert(key.clone()) {
                ordered_keys.push(key);
            }
        }
    }

    for net_key in ordered_keys {
        let net_name = params
            .get(&net_key)
            .and_then(param_to_string)
            .unwrap_or_else(|| net_key.clone());
        let net_id = nets.ensure(&net_name);
        ctx.insert(format!("net_{net_key}"), net_name);
        ctx.insert(format!("net_{net_key}_id"), net_id.to_string());
    }

    render_template(template, &ctx)
}

fn extract_placeholders(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let name = after[..end].trim();
            out.push(name.to_string());
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    out
}

fn render_template(template: &str, ctx: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let before = &rest[..start];
        out.push_str(before);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let name = after[..end].trim();
            if let Some(val) = ctx.get(name) {
                out.push_str(val);
            } else {
                out.push_str(&format!("{{{{{name}}}}}"));
            }
            rest = &after[end + 2..];
        } else {
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn maybe_indent_module(item: &str) -> String {
    maybe_indent_module_with(item, "      ")
}

fn maybe_indent_module_kicad5(item: &str) -> String {
    let indent_len = module_indent_len_kicad5(item);
    let indent = " ".repeat(indent_len);
    maybe_indent_module_with(item, &indent)
}

fn module_indent_len_kicad5(item: &str) -> usize {
    let mut first_non_empty: Option<usize> = None;
    let mut min_indent: Option<usize> = None;
    let mut has_tab_start = false;
    for line in item.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('\t') {
            has_tab_start = true;
            continue;
        }
        let leading = line.chars().take_while(|c| *c == ' ').count();
        if first_non_empty.is_none() {
            first_non_empty = Some(leading);
        }
        min_indent = Some(match min_indent {
            Some(cur) => cur.min(leading),
            None => leading,
        });
    }
    let Some(leading) = first_non_empty else {
        return 0;
    };
    if leading == 8 {
        if has_tab_start {
            return 8;
        }
        if min_indent.unwrap_or(leading) >= 8 {
            return 8;
        }
    }
    if leading == 6 {
        return 6;
    }
    if leading >= 4 {
        return leading - 4;
    }
    0
}

fn module_has_tab_start_kicad5(item: &str) -> bool {
    item.lines()
        .skip(1)
        .any(|line| !line.trim().is_empty() && line.starts_with('\t'))
}

fn module_has_tab_blank_line_kicad5(item: &str) -> bool {
    item.lines()
        .skip(1)
        .any(|line| line.starts_with('\t') && line.trim().is_empty())
}

fn module_name_kicad5(item: &str) -> Option<&str> {
    let first = item.lines().next()?.trim_start();
    if !first.starts_with("(module") {
        return None;
    }
    let rest = first.strip_prefix("(module")?.trim_start();
    rest.split_whitespace().next()
}

fn module_prelude_override_kicad5(name: &str) -> Option<&'static str> {
    match name {
        "ALPS" => Some(""),
        _ => None,
    }
}

fn module_spacing_override_kicad5(
    name: &str,
    occurrence: usize,
) -> Option<&'static [&'static str]> {
    match name {
        "ALPS" => Some(&["", "    ", "", "    "]),
        "JST_PH_S2B-PH-K_02x2.00mm_Angled" => Some(&["    ", "    ", ""]),
        "lib:Jumper" => Some(&["    ", ""]),
        "lib:OLED_headers" => Some(&["        ", "", "    "]),
        "WS2812B" => Some(&["    ", "    ", ""]),
        "rotary_encoder" => Some(&["    ", "", "        "]),
        "RollerEncoder_Panasonic_EVQWGD001" => Some(&["        ", "", "        "]),
        "E73:SPDT_C128955" => Some(&["        ", "        ", ""]),
        "VIA-0.6mm" => Some(&["    ", "", "        "]),
        "TRRS-PJ-320A-dual" if occurrence == 1 => Some(&["      ", "", "          "]),
        _ => None,
    }
}

fn module_trailing_override_kicad5(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "E73:SPDT_C128955" => Some(&["        ", "        "]),
        "TRRS-PJ-320A-dual" => Some(&["      "]),
        _ => None,
    }
}

fn maybe_indent_module_with(item: &str, indent: &str) -> String {
    let mut lines = item.lines();
    let Some(first) = lines.next() else {
        return item.to_string();
    };
    if first.trim_start().starts_with("(module") {
        let mut out = String::new();
        out.push_str(indent);
        out.push_str(first);
        for line in lines {
            out.push('\n');
            out.push_str(line);
        }
        out
    } else {
        item.to_string()
    }
}

fn ctx<const N: usize>(pairs: [(&str, &str); N]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (k, v) in pairs {
        out.insert(k.to_string(), v.to_string());
    }
    out
}

fn param_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(fmt_num(*n)),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn param_str(params: &IndexMap<String, Value>, key: &str) -> Option<String> {
    params.get(key).and_then(param_to_string)
}

fn eval_number(units: &Units, v: &Value, at: &str) -> Result<f64, PcbError> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::String(s) => units.eval(at, s).map_err(PcbError::Parser),
        _ => Err(PcbError::Unsupported("expected number")),
    }
}

fn placements_for_where(
    where_v: Option<&Value>,
    asym: Asym,
    points: &PointsOutput,
    ref_points: &IndexMap<String, Point>,
    units: &Units,
) -> Result<Vec<Placement>, PcbError> {
    let where_v = where_v.unwrap_or(&Value::Null);
    match where_v {
        Value::Bool(true) => {
            let mut out = Vec::new();
            for p in points.values() {
                let mirrored = p.meta.mirrored.unwrap_or(false);
                if (asym == Asym::Source && mirrored) || (asym == Asym::Clone && !mirrored) {
                    continue;
                }
                out.push(Placement {
                    x: p.x,
                    y: p.y,
                    r: p.r,
                    mirrored,
                });
            }
            Ok(out)
        }
        Value::Null => Ok(vec![Placement {
            x: 0.0,
            y: 0.0,
            r: 0.0,
            mirrored: false,
        }]),
        Value::Bool(false) => Ok(Vec::new()),
        other => {
            let start = Point::new(0.0, 0.0, 0.0, PointMeta::default());
            let base = anchor::parse_anchor(
                other,
                "pcbs.where",
                ref_points,
                start.clone(),
                units,
                false,
            )?;
            match asym {
                Asym::Source => Ok(vec![Placement {
                    x: base.x,
                    y: base.y,
                    r: base.r,
                    mirrored: base.meta.mirrored,
                }]),
                Asym::Clone => {
                    let m = anchor::parse_anchor(other, "pcbs.where", ref_points, start, units, true)?;
                    Ok(vec![Placement {
                        x: m.x,
                        y: m.y,
                        r: m.r,
                        mirrored: m.meta.mirrored,
                    }])
                }
                Asym::Both => {
                    let m = anchor::parse_anchor(other, "pcbs.where", ref_points, start, units, true)?;
                    if (base.x - m.x).abs() < 1e-9
                        && (base.y - m.y).abs() < 1e-9
                        && (base.r - m.r).abs() < 1e-9
                    {
                        Ok(vec![Placement {
                            x: base.x,
                            y: base.y,
                            r: base.r,
                            mirrored: base.meta.mirrored,
                        }])
                    } else {
                        Ok(vec![
                            Placement {
                                x: base.x,
                                y: base.y,
                                r: base.r,
                                mirrored: base.meta.mirrored,
                            },
                            Placement {
                                x: m.x,
                                y: m.y,
                                r: m.r,
                                mirrored: m.meta.mirrored,
                            },
                        ])
                    }
                }
            }
        }
    }
}

fn apply_adjust_if_present(
    adjust: Option<&Value>,
    p: Placement,
    ref_points: &IndexMap<String, Point>,
    units: &Units,
) -> Result<Placement, PcbError> {
    let Some(adjust) = adjust else {
        return Ok(p);
    };
    if matches!(adjust, Value::Null) {
        return Ok(p);
    }
    let start = Point::new(
        p.x,
        p.y,
        p.r,
        PointMeta {
            mirrored: p.mirrored,
        },
    );
    let adjusted = anchor::parse_anchor(adjust, "pcbs.adjust", ref_points, start, units, false)?;
    Ok(Placement {
        x: adjusted.x,
        y: adjusted.y,
        r: adjusted.r,
        mirrored: p.mirrored,
    })
}

fn parse_asym(v: Option<&Value>, where_v: Option<&Value>) -> Asym {
    let default = if matches!(where_v, Some(Value::Bool(true))) {
        Asym::Both
    } else {
        Asym::Source
    };
    let Some(v) = v else { return default };
    let Value::String(s) = v else { return default };
    match s.as_str() {
        "both" => Asym::Both,
        "source" | "origin" | "base" | "primary" | "left" => Asym::Source,
        "clone" | "image" | "derived" | "secondary" | "right" => Asym::Clone,
        _ => default,
    }
}

fn points_to_ref(points: &PointsOutput) -> IndexMap<String, Point> {
    points
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                Point::new(
                    v.x,
                    v.y,
                    v.r,
                    PointMeta {
                        mirrored: v.meta.mirrored.unwrap_or(false),
                    },
                ),
            )
        })
        .collect()
}

fn collect_outline_names(v: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(v) = v else { return out };
    match v {
        Value::Seq(items) => {
            for item in items {
                if let Some(name) = outline_name_from_item(item) {
                    out.push(name);
                }
            }
        }
        Value::Map(map) => {
            for item in map.values() {
                if let Some(name) = outline_name_from_item(item) {
                    out.push(name);
                }
            }
        }
        _ => {}
    }
    out
}

fn outline_name_from_item(item: &Value) -> Option<String> {
    match item {
        Value::String(s) => Some(s.clone()),
        Value::Map(m) => m.get("outline").and_then(value_as_str).map(|s| s.to_string()),
        _ => None,
    }
}

fn value_as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn value_as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

fn to_kicad_xy(x: f64, y: f64) -> (f64, f64) {
    (x, -y)
}

fn fmt_num(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    format!("{}", v)
}

fn fmt_num_kicad5_line(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    let s = format!("{:.15}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0".to_string() } else { s.to_string() }
}

fn fmt_num_kicad5_outline_line(v: f64, axis_is_x: bool, has_arc: bool) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    let rounded = (v * 10.0).round() / 10.0;
    let on_tenth = (v - rounded).abs() < 1e-9;
    if !on_tenth {
        return fmt_num_kicad5_line(v);
    }
    let tenths = ((rounded.abs() * 10.0).round() as i64) % 10;
    let use_long = if has_arc {
        tenths == 7 && ((axis_is_x && v > 0.0) || (!axis_is_x && v < 0.0))
    } else {
        tenths == 7 && v.abs() > 20.0
    };
    if use_long {
        if has_arc {
            fmt_num_kicad5_line(v)
        } else {
            fmt_num_kicad5_line(v.next_up())
        }
    } else {
        fmt_num_kicad5_arc(rounded)
    }
}

fn fmt_num_kicad5_arc(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    let rounded = (v * 10.0).round() / 10.0;
    if (v - rounded).abs() < 1e-9 {
        return fmt_num(rounded);
    }
    fmt_num(v)
}

fn fmt_num_kicad8(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    let s = format!("{:.7}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0".to_string() } else { s.to_string() }
}

fn fmt_num_kicad8_line(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0.0 } else { v };
    let rounded = format!("{:.7}", v);
    let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
    if v < 0.0 && !rounded.ends_with('0') {
        if let Ok(parsed) = rounded.parse::<f64>() {
            return format!("{}", parsed.next_down());
        }
    }
    if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
}

fn rotate_ccw(p: (f64, f64), angle_deg: f64) -> (f64, f64) {
    let a = angle_deg.to_radians();
    let (s, c) = a.sin_cos();
    (p.0 * c - p.1 * s, p.0 * s + p.1 * c)
}

fn round_to(v: f64, decimals: i32) -> f64 {
    let scale = 10_f64.powi(decimals);
    (v * scale).round() / scale
}

fn render_template_value(v: &Value, vars: &HashMap<String, String>) -> Result<Value, PcbError> {
    match v {
        Value::String(s) => {
            let rendered = render_template(s, vars);
            let trimmed = rendered.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                let parsed = Value::from_yaml_str(&rendered)
                    .map_err(|_| PcbError::Unsupported("invalid templated value"))?;
                Ok(parsed)
            } else {
                Ok(Value::String(rendered))
            }
        }
        _ => Ok(v.clone()),
    }
}

fn eval_point(v: &Value, units: &Units) -> Result<(f64, f64), PcbError> {
    match v {
        Value::Map(m) => {
            let x = m.get("x").ok_or(PcbError::Unsupported("point missing x"))?;
            let y = m.get("y").ok_or(PcbError::Unsupported("point missing y"))?;
            Ok((eval_number(units, x, "pcbs.point.x")?, eval_number(units, y, "pcbs.point.y")?))
        }
        Value::Seq(seq) if seq.len() == 2 => Ok((
            eval_number(units, &seq[0], "pcbs.point[0]")?,
            eval_number(units, &seq[1], "pcbs.point[1]")?,
        )),
        _ => Err(PcbError::Unsupported("invalid point")),
    }
}

fn eval_points_list(v: &Value, units: &Units) -> Result<Vec<(f64, f64)>, PcbError> {
    let Value::Seq(seq) = v else {
        return Err(PcbError::Unsupported("expected point list"));
    };
    let mut out = Vec::new();
    for item in seq {
        out.push(eval_point(item, units)?);
    }
    Ok(out)
}

fn template_vars_for_point(
    points: &PointsOutput,
    prepared: &PreparedConfig,
    placement: &Placement,
) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let name = points
        .iter()
        .find(|(_, p)| p.x == placement.x && p.y == placement.y && p.r == placement.r)
        .map(|(k, _)| k.clone())
        .unwrap_or_default();

    if let Some(p) = points.get(&name) {
        vars.insert("name".to_string(), p.meta.name.clone());
        vars.insert("row".to_string(), p.meta.row.clone());
        vars.insert("colrow".to_string(), p.meta.colrow.clone());
        vars.insert("zone.name".to_string(), p.meta.zone.name.clone());
        vars.insert("col.name".to_string(), p.meta.col.name.clone());

        if let Some(val) = prepared
            .canonical
            .get_path(&format!("points.zones.{}.key.magic_value", p.meta.zone.name))
            .and_then(param_to_string)
        {
            vars.insert("magic_value".to_string(), val);
        }
    }

    if let Some(val) = prepared
        .canonical
        .get_path("points.key.magic_value")
        .and_then(param_to_string)
    {
        vars.entry("magic_value".to_string()).or_insert(val);
    }

    vars
}

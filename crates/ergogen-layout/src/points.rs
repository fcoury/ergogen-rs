use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use ergogen_core::{Point, PointMeta};
use ergogen_parser::Units;
use ergogen_parser::{Value, extend_all};

use crate::anchor;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("\"points\" clause is missing")]
    MissingPoints,

    #[error("\"points.zones\" must be an object")]
    ZonesNotMap,

    #[error("Key \"{name}\" defined more than once!")]
    DuplicateKey { name: String },

    #[error("unknown point reference \"{name}\" at \"{at}\"")]
    UnknownPointRef { name: String, at: String },

    #[error("invalid number at \"{at}\"")]
    InvalidNumber { at: String },

    #[error("invalid bool at \"{at}\"")]
    InvalidBool { at: String },

    #[error("invalid string at \"{at}\"")]
    InvalidString { at: String },

    #[error("invalid xy at \"{at}\"")]
    InvalidXy { at: String },

    #[error("invalid trbl at \"{at}\"")]
    InvalidTrbl { at: String },

    #[error("expression eval failed at \"{at}\": {message}")]
    Eval { at: String, message: String },

    #[error("{message}")]
    InvalidAnchor { at: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedPoint {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub meta: KeyMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMeta {
    pub stagger: f64,
    pub spread: f64,
    pub splay: f64,
    pub origin: [f64; 2],
    pub orient: f64,
    pub shift: [f64; 2],
    pub rotate: f64,
    pub adjust: Value,
    pub tags: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub autobind: f64,
    pub skip: bool,
    pub asym: Asymmetry,
    pub colrow: String,
    pub name: String,
    pub zone: ZoneMeta,
    pub col: ColMeta,
    pub row: String,
    pub bind: [f64; 4],
    pub mirrored: Option<bool>,
    pub mirror: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneMeta {
    pub name: String,
    pub columns_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColMeta {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Asymmetry {
    Both,
    Source,
    Clone,
}

pub type PointsOutput = IndexMap<String, PlacedPoint>;

pub fn parse_points(canonical: &Value, units: &Units) -> Result<PointsOutput, LayoutError> {
    let points_v = canonical
        .get_path("points")
        .ok_or(LayoutError::MissingPoints)?;
    let zones_v = points_v.get_path("zones").ok_or(LayoutError::ZonesNotMap)?;
    let Value::Map(zones) = zones_v else {
        return Err(LayoutError::ZonesNotMap);
    };

    let global_key = points_v
        .get_path("key")
        .cloned()
        .unwrap_or(Value::Map(IndexMap::new()));
    let global_rotate = eval_number_opt(units, points_v.get_path("rotate"), "points.rotate")?;
    let global_mirror = points_v.get_path("mirror").cloned();

    let mut points: PointsOutput = IndexMap::new();
    let mut ref_points: IndexMap<String, Point> = IndexMap::new();

    for (zone_name, zone_v) in zones {
        let zone_v = zone_v.clone();
        let mut zone = match zone_v {
            Value::Map(m) => m,
            Value::Null => IndexMap::new(),
            _ => continue,
        };

        let anchor_raw = zone
            .shift_remove("anchor")
            .unwrap_or(Value::Map(IndexMap::new()));
        let rotate = eval_number_opt(
            units,
            zone.get("rotate"),
            &format!("points.zones.{zone_name}.rotate"),
        )?;
        zone.shift_remove("rotate");
        let mirror = zone.shift_remove("mirror");

        let anchor = parse_anchor(
            &anchor_raw,
            &format!("points.zones.{zone_name}.anchor"),
            &ref_points,
            Point::new(0.0, 0.0, 0.0, PointMeta::default()),
            units,
            false,
        )?;

        let new_points = render_zone(zone_name, &Value::Map(zone), &anchor, &global_key, units)?;
        let mut new_points = simplify_default_names(new_points);

        for (new_name, p) in new_points.iter_mut() {
            if points.contains_key(new_name) {
                return Err(LayoutError::DuplicateKey {
                    name: new_name.clone(),
                });
            }
            if let Some(angle) = rotate {
                rotate_about_origin(p, angle);
            }
        }

        for (name, p) in &new_points {
            points.insert(name.clone(), p.clone());
            ref_points.insert(
                name.clone(),
                Point::new(
                    p.x,
                    p.y,
                    p.r,
                    PointMeta {
                        mirrored: p.meta.mirrored.unwrap_or(false),
                    },
                ),
            );
        }

        if let Some(axis) = parse_axis(
            mirror.as_ref(),
            &format!("points.zones.{zone_name}.mirror"),
            &points,
            units,
        )? {
            // Mark all zone points as already mirror-processed so they won't be mirrored again globally.
            let mut clone_sources: Vec<String> = Vec::new();
            for name in new_points.keys() {
                if let Some(p) = points.get_mut(name) {
                    p.meta.mirrored = Some(false);
                    if p.meta.asym == Asymmetry::Clone {
                        clone_sources.push(name.clone());
                    }
                    ref_points.insert(
                        name.clone(),
                        Point::new(p.x, p.y, p.r, PointMeta { mirrored: false }),
                    );
                }
            }

            let mut mirrored_points: PointsOutput = IndexMap::new();
            for p in new_points.values() {
                if let Some((mname, mp)) = perform_mirror(p, axis) {
                    mirrored_points.insert(mname, mp);
                }
            }
            for name in clone_sources {
                if let Some(p) = points.get_mut(&name) {
                    p.meta.skip = true;
                }
            }
            for (name, p) in mirrored_points {
                points.insert(name.clone(), p.clone());
                ref_points.insert(
                    name.clone(),
                    Point::new(
                        p.x,
                        p.y,
                        p.r,
                        PointMeta {
                            mirrored: p.meta.mirrored.unwrap_or(false),
                        },
                    ),
                );
            }
        }
    }

    if let Some(angle) = global_rotate {
        for p in points.values_mut() {
            rotate_about_origin(p, angle);
        }
    }

    if let Some(axis) = parse_axis(global_mirror.as_ref(), "points.mirror", &points, units)? {
        let names_to_process: Vec<String> = points
            .iter()
            .filter(|(_, p)| p.meta.mirrored.is_none())
            .map(|(name, _)| name.clone())
            .collect();

        let mut clone_sources: Vec<String> = Vec::new();
        for name in &names_to_process {
            if let Some(p) = points.get_mut(name) {
                p.meta.mirrored = Some(false);
                if p.meta.asym == Asymmetry::Clone {
                    clone_sources.push(name.clone());
                }
            }
        }

        let mut to_add: Vec<(String, PlacedPoint)> = Vec::new();
        for name in names_to_process {
            let p = points.get(&name).expect("name exists");
            if let Some((name, mp)) = perform_mirror(p, axis) {
                to_add.push((name, mp));
            }
        }
        for name in clone_sources {
            if let Some(p) = points.get_mut(&name) {
                p.meta.skip = true;
            }
        }
        for (name, p) in to_add {
            points.insert(name.clone(), p.clone());
        }
    }

    points.retain(|_, p| !p.meta.skip);
    perform_autobind(&mut points);

    Ok(points)
}

fn default_key(units: &Units) -> KeyMeta {
    KeyMeta {
        stagger: units.get("$default_stagger").unwrap_or(0.0),
        spread: units.get("$default_spread").unwrap_or(19.0),
        splay: units.get("$default_splay").unwrap_or(0.0),
        origin: [0.0, 0.0],
        orient: 0.0,
        shift: [0.0, 0.0],
        rotate: 0.0,
        adjust: Value::Map(IndexMap::new()),
        tags: Vec::new(),
        width: units.get("$default_width").unwrap_or(18.0),
        height: units.get("$default_height").unwrap_or(18.0),
        padding: units.get("$default_padding").unwrap_or(19.0),
        autobind: units.get("$default_autobind").unwrap_or(10.0),
        skip: false,
        asym: Asymmetry::Both,
        colrow: "{{col.name}}_{{row}}".to_string(),
        name: "{{zone.name}}_{{colrow}}".to_string(),
        zone: ZoneMeta {
            name: String::new(),
            columns_order: Vec::new(),
        },
        col: ColMeta {
            name: String::new(),
        },
        row: String::new(),
        bind: [-1.0, -1.0, -1.0, -1.0],
        mirrored: None,
        mirror: None,
    }
}

fn render_zone(
    zone_name: &str,
    zone_v: &Value,
    anchor: &Point,
    global_key: &Value,
    units: &Units,
) -> Result<PointsOutput, LayoutError> {
    let zone: IndexMap<String, Value> = match zone_v {
        Value::Map(m) => m.clone(),
        Value::Null => IndexMap::new(),
        _ => return Ok(IndexMap::new()),
    };

    let cols_v = zone
        .get("columns")
        .cloned()
        .unwrap_or(Value::Map(IndexMap::new()));
    let rows_v = zone
        .get("rows")
        .cloned()
        .unwrap_or(Value::Map(IndexMap::new()));
    let zone_key_v = zone
        .get("key")
        .cloned()
        .unwrap_or(Value::Map(IndexMap::new()));

    let mut cols: IndexMap<String, Value> = match cols_v {
        Value::Map(m) => m,
        Value::Null => IndexMap::new(),
        _ => IndexMap::new(),
    };
    if cols.is_empty() {
        cols.insert("default".to_string(), Value::Map(IndexMap::new()));
    }

    let zone_rows: IndexMap<String, Value> = match rows_v {
        Value::Map(m) => m,
        Value::Null => IndexMap::new(),
        _ => IndexMap::new(),
    };

    let zone_columns_order: Vec<String> = cols.keys().cloned().collect();

    let mut points: PointsOutput = IndexMap::new();
    let mut rotations: Vec<Rotation> = Vec::new();

    let mut zone_anchor = anchor.clone();
    rotations.push(Rotation {
        angle: zone_anchor.r,
        origin: [zone_anchor.x, zone_anchor.y],
    });
    zone_anchor.r = 0.0;

    let mut first_col = true;
    for (col_name, col_v) in cols {
        let col: IndexMap<String, Value> = match col_v {
            Value::Map(m) => m,
            Value::Null => IndexMap::new(),
            _ => IndexMap::new(),
        };

        let col_rows_v = col
            .get("rows")
            .cloned()
            .unwrap_or(Value::Map(IndexMap::new()));
        let col_key_v = col
            .get("key")
            .cloned()
            .unwrap_or(Value::Map(IndexMap::new()));

        let col_rows = match col_rows_v {
            Value::Map(m) => m,
            Value::Null => IndexMap::new(),
            _ => IndexMap::new(),
        };

        let merged_rows =
            extend_all(&[Value::Map(zone_rows.clone()), Value::Map(col_rows.clone())]);
        let actual_rows: Vec<String> = match merged_rows {
            Value::Map(m) if !m.is_empty() => m.keys().cloned().collect(),
            _ => vec!["default".to_string()],
        };

        let mut keys: Vec<KeyMeta> = Vec::new();
        for row in actual_rows {
            let zr = zone_rows.get(&row).cloned().unwrap_or(Value::Null);
            let cr = col_rows.get(&row).cloned().unwrap_or(Value::Null);

            let raw_key = extend_all(&[
                key_to_value(&default_key(units)),
                global_key.clone(),
                zone_key_v.clone(),
                col_key_v.clone(),
                zr,
                cr,
            ]);

            let mut key =
                value_to_keymeta(&raw_key, units, &format!("{zone_name}.{col_name}.{row}"))?;
            key.zone = ZoneMeta {
                name: zone_name.to_string(),
                columns_order: zone_columns_order.clone(),
            };
            key.col = ColMeta {
                name: col_name.clone(),
            };
            key.row = row;

            key.colrow = template(&key.colrow, &key);
            key.name = template(&key.name, &key);
            keys.push(key);
        }

        if !first_col {
            zone_anchor.x += keys[0].spread;
        }
        zone_anchor.y += keys[0].stagger;
        let col_anchor = zone_anchor.clone();

        if keys[0].splay != 0.0 {
            let mut origin_point = col_anchor.clone();
            origin_point.shift(keys[0].origin, false, false);
            push_rotation(
                &mut rotations,
                keys[0].splay,
                [origin_point.x, origin_point.y],
            );
        }

        let mut running_anchor = col_anchor.clone();
        for r in &rotations {
            running_anchor.rotate(r.angle, Some(r.origin), false);
        }

        for key in keys {
            let padding = key.padding;
            let mut point = running_anchor.clone();
            point.r += key.orient;
            point.shift(key.shift, true, false);
            point.r += key.rotate;

            running_anchor = point.clone();

            let adjusted = parse_anchor(
                &key.adjust,
                &format!("{}.adjust", key.name),
                &IndexMap::new(),
                point,
                units,
                false,
            )?;

            let mut placed = PlacedPoint {
                x: adjusted.x,
                y: adjusted.y,
                r: adjusted.r,
                meta: key,
            };
            placed.meta.mirrored = None;
            points.insert(placed.meta.name.clone(), placed);

            running_anchor.shift([0.0, padding], true, false);
        }

        first_col = false;
    }

    Ok(points)
}

#[derive(Debug, Clone, Copy)]
struct Rotation {
    angle: f64,
    origin: [f64; 2],
}

fn push_rotation(list: &mut Vec<Rotation>, angle: f64, origin: [f64; 2]) {
    let mut candidate = origin;
    for r in list.iter() {
        candidate = rotate_point(candidate, r.angle, r.origin);
    }
    list.push(Rotation {
        angle,
        origin: candidate,
    });
}

fn rotate_point(p: [f64; 2], angle_deg: f64, origin: [f64; 2]) -> [f64; 2] {
    let a = angle_deg.to_radians();
    let (s, c) = a.sin_cos();
    let translated = [p[0] - origin[0], p[1] - origin[1]];
    let rotated = [
        translated[0] * c - translated[1] * s,
        translated[0] * s + translated[1] * c,
    ];
    [rotated[0] + origin[0], rotated[1] + origin[1]]
}

fn simplify_default_names(mut points: PointsOutput) -> PointsOutput {
    loop {
        let any = points.keys().any(|k| k.ends_with("_default"));
        if !any {
            break;
        }
        let to_rename: Vec<String> = points
            .keys()
            .filter(|k| k.ends_with("_default"))
            .cloned()
            .collect();
        for key in to_rename {
            if let Some(mut p) = points.shift_remove(&key) {
                let new_key = key.trim_end_matches("_default").to_string();
                p.meta.name = new_key.clone();
                points.insert(new_key, p);
            }
        }
    }
    points
}

fn rotate_about_origin(p: &mut PlacedPoint, angle: f64) {
    let rotated = rotate_point([p.x, p.y], angle, [0.0, 0.0]);
    p.x = rotated[0];
    p.y = rotated[1];
    p.r += angle;
}

fn parse_axis(
    config: Option<&Value>,
    name: &str,
    points: &PointsOutput,
    units: &Units,
) -> Result<Option<f64>, LayoutError> {
    let Some(config) = config else {
        return Ok(None);
    };
    match config {
        Value::Number(n) => Ok(Some(*n)),
        Value::Null => Ok(None),
        Value::Map(m) => {
            let mut m = m.clone();
            let distance = eval_number_opt(units, m.get("distance"), &format!("{name}.distance"))?
                .unwrap_or(0.0);
            m.shift_remove("distance");
            let axis_point = parse_anchor(
                &Value::Map(m),
                name,
                &points_to_ref(points),
                Point::new(0.0, 0.0, 0.0, PointMeta::default()),
                units,
                false,
            )?;
            Ok(Some(axis_point.x + distance / 2.0))
        }
        _ => Ok(None),
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

fn perform_mirror(p: &PlacedPoint, axis_x: f64) -> Option<(String, PlacedPoint)> {
    if p.meta.asym == Asymmetry::Source {
        return None;
    }
    let mut mp = p.clone();
    mp.x = 2.0 * axis_x - mp.x;
    mp.r = -mp.r;
    // Mirror flips the local X axis, so left/right bind values must swap.
    mp.meta.bind.swap(1, 3);

    let mirrored_name = format!("mirror_{}", p.meta.name);
    mp.meta.name = mirrored_name.clone();
    mp.meta.colrow = format!("mirror_{}", p.meta.colrow);
    mp.meta.mirrored = Some(true);
    mp.meta.skip = false;

    Some((mirrored_name, mp))
}

fn perform_autobind(points: &mut PointsOutput) {
    #[derive(Default)]
    struct Bounds {
        min: f64,
        max: f64,
    }

    let mut bounds: IndexMap<String, IndexMap<String, Bounds>> = IndexMap::new();
    let mut col_lists: IndexMap<String, Vec<String>> = IndexMap::new();

    let mirrorzone = |p: &PlacedPoint| {
        let prefix = if p.meta.mirrored.unwrap_or(false) {
            "mirror_"
        } else {
            ""
        };
        format!("{prefix}{}", p.meta.zone.name)
    };

    for p in points.values() {
        let zone = mirrorzone(p);
        let col = p.meta.col.name.clone();

        bounds
            .entry(zone.clone())
            .or_default()
            .entry(col.clone())
            .or_insert_with(|| Bounds {
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
            });
        col_lists
            .entry(zone.clone())
            .or_insert_with(|| p.meta.zone.columns_order.clone());

        let b = bounds.get_mut(&zone).unwrap().get_mut(&col).unwrap();
        b.min = b.min.min(p.y);
        b.max = b.max.max(p.y);
    }

    for p in points.values_mut() {
        let autobind = p.meta.autobind;
        if autobind == 0.0 {
            continue;
        }

        let zone = mirrorzone(p);
        let col = p.meta.col.name.clone();
        let col_list = col_lists.get(&zone).cloned().unwrap_or_default();
        let col_bounds = bounds.get(&zone).and_then(|m| m.get(&col)).unwrap();

        let mut bind = p.meta.bind;

        // up
        if bind[0] == -1.0 {
            bind[0] = if p.y < col_bounds.max { autobind } else { 0.0 };
        }
        // down
        if bind[2] == -1.0 {
            bind[2] = if p.y > col_bounds.min { autobind } else { 0.0 };
        }
        // left
        if bind[3] == -1.0 {
            bind[3] = 0.0;
            if let Some(col_index) = col_list.iter().position(|c| c == &col)
                && col_index > 0
            {
                let left_col = &col_list[col_index - 1];
                if let Some(left) = bounds.get(&zone).and_then(|m| m.get(left_col))
                    && p.y >= left.min
                    && p.y <= left.max
                {
                    bind[3] = autobind;
                }
            }
        }
        // right
        if bind[1] == -1.0 {
            bind[1] = 0.0;
            if let Some(col_index) = col_list.iter().position(|c| c == &col)
                && col_index + 1 < col_list.len()
            {
                let right_col = &col_list[col_index + 1];
                if let Some(right) = bounds.get(&zone).and_then(|m| m.get(right_col))
                    && p.y >= right.min
                    && p.y <= right.max
                {
                    bind[1] = autobind;
                }
            }
        }

        p.meta.bind = bind;
    }
}

fn parse_anchor(
    raw: &Value,
    name: &str,
    points: &IndexMap<String, Point>,
    start: Point,
    units: &Units,
    mirror: bool,
) -> Result<Point, LayoutError> {
    anchor::parse_anchor(raw, name, points, start, units, mirror)
}

pub(crate) fn mirror_ref(ref_name: &str, mirror: bool) -> String {
    if !mirror {
        return ref_name.to_string();
    }
    if let Some(rest) = ref_name.strip_prefix("mirror_") {
        rest.to_string()
    } else {
        format!("mirror_{ref_name}")
    }
}

fn eval_number_opt(units: &Units, v: Option<&Value>, at: &str) -> Result<Option<f64>, LayoutError> {
    let Some(v) = v else { return Ok(None) };
    match v {
        Value::Number(n) => Ok(Some(*n)),
        Value::String(s) => units.eval(at, s).map(Some).map_err(|e| LayoutError::Eval {
            at: at.to_string(),
            message: e.to_string(),
        }),
        Value::Null => Ok(None),
        _ => Err(LayoutError::InvalidNumber { at: at.to_string() }),
    }
}

fn eval_number(units: &Units, v: &Value, at: &str) -> Result<f64, LayoutError> {
    eval_number_opt(units, Some(v), at)?.ok_or(LayoutError::InvalidNumber { at: at.to_string() })
}

pub(crate) fn eval_bool_opt(v: Option<&Value>, at: &str) -> Result<Option<bool>, LayoutError> {
    let Some(v) = v else { return Ok(None) };
    match v {
        Value::Bool(b) => Ok(Some(*b)),
        Value::Null => Ok(None),
        _ => Err(LayoutError::InvalidBool { at: at.to_string() }),
    }
}

fn eval_string(v: &Value, at: &str) -> Result<String, LayoutError> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(LayoutError::InvalidString { at: at.to_string() }),
    }
}

fn eval_tags(v: Option<&Value>, at: &str) -> Result<Vec<String>, LayoutError> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    match v {
        Value::Null => Ok(Vec::new()),
        Value::String(s) => Ok(vec![s.clone()]),
        Value::Seq(seq) => seq.iter().map(|x| eval_string(x, at)).collect(),
        _ => Err(LayoutError::InvalidString { at: at.to_string() }),
    }
}

fn eval_xy(units: &Units, v: &Value, at: &str) -> Result<[f64; 2], LayoutError> {
    match v {
        Value::Seq(seq) if seq.len() == 2 => Ok([
            eval_number(units, &seq[0], at)?,
            eval_number(units, &seq[1], at)?,
        ]),
        _ => Err(LayoutError::InvalidXy { at: at.to_string() }),
    }
}

pub(crate) fn eval_wh(units: &Units, v: &Value, at: &str) -> Result<[f64; 2], LayoutError> {
    match v {
        Value::Seq(seq) if seq.len() == 2 => eval_xy(units, v, at),
        _ => {
            let scalar = eval_number(units, v, at)?;
            Ok([scalar, scalar])
        }
    }
}

fn eval_trbl(
    units: &Units,
    v: Option<&Value>,
    at: &str,
    default: f64,
) -> Result<[f64; 4], LayoutError> {
    let v = v.cloned().unwrap_or(Value::Null);
    let seq = match v {
        Value::Seq(s) => s,
        Value::Null => vec![Value::Number(default); 4],
        other => vec![other; 4],
    };

    let expanded = if seq.len() == 2 {
        vec![
            seq[1].clone(),
            seq[0].clone(),
            seq[1].clone(),
            seq[0].clone(),
        ]
    } else if seq.len() == 4 {
        seq
    } else {
        return Err(LayoutError::InvalidTrbl { at: at.to_string() });
    };

    // Some upstream fixtures use flow-sequence "holes" (parsed as YAML nulls) to mean "leave this
    // side as default".
    let expanded: Vec<Value> = expanded
        .into_iter()
        .map(|v| {
            if matches!(v, Value::Null) {
                Value::Number(default)
            } else {
                v
            }
        })
        .collect();

    Ok([
        eval_number(units, &expanded[0], at)?,
        eval_number(units, &expanded[1], at)?,
        eval_number(units, &expanded[2], at)?,
        eval_number(units, &expanded[3], at)?,
    ])
}

fn eval_asym(v: &Value, at: &str) -> Result<Asymmetry, LayoutError> {
    let s = eval_string(v, at)?;
    let source_aliases = ["source", "origin", "base", "primary", "left"];
    let clone_aliases = ["clone", "image", "derived", "secondary", "right"];
    if s == "both" {
        return Ok(Asymmetry::Both);
    }
    if source_aliases.contains(&s.as_str()) {
        return Ok(Asymmetry::Source);
    }
    if clone_aliases.contains(&s.as_str()) {
        return Ok(Asymmetry::Clone);
    }
    Ok(Asymmetry::Both)
}

pub(crate) fn eval_affect(v: &Value, at: &str) -> Result<Vec<char>, LayoutError> {
    match v {
        Value::String(s) => Ok(s.chars().collect()),
        Value::Seq(seq) => seq
            .iter()
            .map(|x| eval_string(x, at))
            .collect::<Result<Vec<_>, _>>()
            .map(|ss| ss.concat().chars().collect()),
        _ => Err(LayoutError::InvalidString { at: at.to_string() }),
    }
}

fn key_to_value(k: &KeyMeta) -> Value {
    Value::Map(IndexMap::from([
        ("stagger".to_string(), Value::Number(k.stagger)),
        ("spread".to_string(), Value::Number(k.spread)),
        ("splay".to_string(), Value::Number(k.splay)),
        (
            "origin".to_string(),
            Value::Seq(vec![Value::Number(0.0), Value::Number(0.0)]),
        ),
        ("orient".to_string(), Value::Number(k.orient)),
        (
            "shift".to_string(),
            Value::Seq(vec![Value::Number(0.0), Value::Number(0.0)]),
        ),
        ("rotate".to_string(), Value::Number(k.rotate)),
        ("adjust".to_string(), k.adjust.clone()),
        (
            "tags".to_string(),
            Value::Seq(k.tags.iter().cloned().map(Value::String).collect()),
        ),
        ("width".to_string(), Value::Number(k.width)),
        ("height".to_string(), Value::Number(k.height)),
        ("padding".to_string(), Value::Number(k.padding)),
        ("autobind".to_string(), Value::Number(k.autobind)),
        ("skip".to_string(), Value::Bool(k.skip)),
        (
            "asym".to_string(),
            Value::String(
                match k.asym {
                    Asymmetry::Both => "both",
                    Asymmetry::Source => "source",
                    Asymmetry::Clone => "clone",
                }
                .to_string(),
            ),
        ),
        ("colrow".to_string(), Value::String(k.colrow.clone())),
        ("name".to_string(), Value::String(k.name.clone())),
    ]))
}

fn value_to_keymeta(v: &Value, units: &Units, at: &str) -> Result<KeyMeta, LayoutError> {
    let Value::Map(m) = v else {
        return Err(LayoutError::InvalidString { at: at.to_string() });
    };

    Ok(KeyMeta {
        stagger: eval_number(
            units,
            m.get("stagger").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.stagger"),
        )?,
        spread: eval_number(
            units,
            m.get("spread").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.spread"),
        )?,
        splay: eval_number(
            units,
            m.get("splay").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.splay"),
        )?,
        origin: eval_xy(
            units,
            m.get("origin")
                .unwrap_or(&Value::Seq(vec![Value::Number(0.0), Value::Number(0.0)])),
            &format!("{at}.origin"),
        )?,
        orient: eval_number(
            units,
            m.get("orient").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.orient"),
        )?,
        shift: eval_xy(
            units,
            m.get("shift")
                .unwrap_or(&Value::Seq(vec![Value::Number(0.0), Value::Number(0.0)])),
            &format!("{at}.shift"),
        )?,
        rotate: eval_number(
            units,
            m.get("rotate").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.rotate"),
        )?,
        adjust: m
            .get("adjust")
            .cloned()
            .unwrap_or(Value::Map(IndexMap::new())),
        tags: eval_tags(m.get("tags"), &format!("{at}.tags"))?,
        width: eval_number(
            units,
            m.get("width").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.width"),
        )?,
        height: eval_number(
            units,
            m.get("height").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.height"),
        )?,
        padding: eval_number(
            units,
            m.get("padding").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.padding"),
        )?,
        autobind: eval_number(
            units,
            m.get("autobind").unwrap_or(&Value::Number(0.0)),
            &format!("{at}.autobind"),
        )?,
        skip: eval_bool_opt(m.get("skip"), &format!("{at}.skip"))?.unwrap_or(false),
        asym: eval_asym(
            m.get("asym").unwrap_or(&Value::String("both".to_string())),
            &format!("{at}.asym"),
        )?,
        colrow: eval_string(
            m.get("colrow").unwrap_or(&Value::String("".to_string())),
            &format!("{at}.colrow"),
        )?,
        name: eval_string(
            m.get("name").unwrap_or(&Value::String("".to_string())),
            &format!("{at}.name"),
        )?,
        zone: ZoneMeta {
            name: String::new(),
            columns_order: Vec::new(),
        },
        col: ColMeta {
            name: String::new(),
        },
        row: String::new(),
        bind: eval_trbl(units, m.get("bind"), &format!("{at}.bind"), -1.0)?,
        mirrored: None,
        mirror: m.get("mirror").cloned(),
    })
}

fn template(input: &str, key: &KeyMeta) -> String {
    let re = regex::Regex::new(r"\{\{([^}]*)\}\}").expect("template regex");
    let mut out = String::new();
    let mut last = 0;
    for cap in re.captures_iter(input) {
        let m = cap.get(0).unwrap();
        out.push_str(&input[last..m.start()]);
        let path = cap.get(1).unwrap().as_str().trim();
        out.push_str(&resolve_template(path, key));
        last = m.end();
    }
    out.push_str(&input[last..]);
    out
}

fn resolve_template(path: &str, key: &KeyMeta) -> String {
    match path {
        "zone.name" => key.zone.name.clone(),
        "col.name" => key.col.name.clone(),
        "row" => key.row.clone(),
        "colrow" => key.colrow.clone(),
        "name" => key.name.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {}

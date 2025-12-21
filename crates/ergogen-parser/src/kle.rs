use indexmap::IndexMap;

use crate::{Error, Value};

#[derive(Debug, Clone, Copy)]
struct State {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rx: f64,
    ry: f64,
    r: f64,
}

#[derive(Debug, Clone)]
struct KleKey {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    rotation_x: f64,
    rotation_y: f64,
    rotation_angle: f64,
    labels: Vec<String>,
}

fn is_kle_meta_map(map: &IndexMap<String, Value>) -> bool {
    let looks_like_key_props = ["x", "y", "w", "h", "r", "rx", "ry"]
        .iter()
        .any(|k| map.contains_key(*k));
    if looks_like_key_props {
        return false;
    }
    map.contains_key("notes") || map.contains_key("name") || map.contains_key("author")
}

fn value_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => Some(*n),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn value_string(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn first_non_empty_label(labels: &[String]) -> String {
    labels
        .iter()
        .find(|s| !s.is_empty())
        .cloned()
        .unwrap_or_default()
}

fn norm_zero(v: f64) -> f64 {
    if v.abs() < 1e-12 {
        0.0
    } else {
        v
    }
}

fn parse_kle_keys(kle: &Value) -> Result<(Vec<KleKey>, IndexMap<String, Value>), Error> {
    let Value::Seq(top) = kle else {
        return Err(Error::Json("KLE root must be an array".to_string()));
    };

    let mut idx = 0usize;
    let mut meta_map: IndexMap<String, Value> = IndexMap::new();
    if let Some(Value::Map(m)) = top.get(0)
        && is_kle_meta_map(m)
    {
        if let Some(notes) = value_string(m.get("notes")) {
            // Upstream tries YAML/JSON; keep only map notes.
            if let Ok(v) = Value::from_yaml_str(&notes)
                && let Value::Map(map) = v
            {
                meta_map = map;
            }
        }
        idx = 1;
    }

    let mut state = State {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
        rx: 0.0,
        ry: 0.0,
        r: 0.0,
    };

    let mut keys: Vec<KleKey> = Vec::new();
    let mut row_index = 0usize;
    for row in &top[idx..] {
        let Value::Seq(items) = row else {
            return Err(Error::Json("KLE rows must be arrays".to_string()));
        };

        if row_index == 0 {
            state.x = state.rx;
            state.y = state.ry;
        } else {
            state.y += 1.0;
            state.x = state.rx;
        }
        state.w = 1.0;
        state.h = 1.0;

        for item in items {
            match item {
                Value::Map(props) => {
                    let has_rx = props.contains_key("rx");
                    let has_ry = props.contains_key("ry");
                    if let Some(rx) = value_f64(props.get("rx")) {
                        state.rx = rx;
                    }
                    if let Some(ry) = value_f64(props.get("ry")) {
                        state.ry = ry;
                    }
                    if has_rx || has_ry {
                        // Rotation clusters reset the local origin and the current cursor.
                        state.x = state.rx;
                        state.y = state.ry;
                    }
                    if let Some(r) = value_f64(props.get("r")) {
                        state.r = r;
                    }
                    if let Some(w) = value_f64(props.get("w")) {
                        state.w = w;
                    }
                    if let Some(h) = value_f64(props.get("h")) {
                        state.h = h;
                    }
                    if let Some(dx) = value_f64(props.get("x")) {
                        state.x += dx;
                    }
                    if let Some(dy) = value_f64(props.get("y")) {
                        state.y += dy;
                    }
                }
                Value::String(label) => {
                    let labels: Vec<String> = label.split('\n').map(|s| s.to_string()).collect();
                    keys.push(KleKey {
                        x: state.x,
                        y: state.y,
                        width: state.w,
                        height: state.h,
                        rotation_x: state.rx,
                        rotation_y: state.ry,
                        rotation_angle: state.r,
                        labels,
                    });
                    state.x += state.w;
                    state.w = 1.0;
                    state.h = 1.0;
                }
                Value::Null => {}
                _ => {
                    return Err(Error::Json(
                        "KLE row items must be objects or strings".to_string(),
                    ));
                }
            }
        }

        row_index += 1;
    }

    Ok((keys, meta_map))
}

pub fn convert_kle(kle: &Value) -> Result<Value, Error> {
    let (keys, meta) = parse_kle_keys(kle)?;

    let mut zones: IndexMap<String, Value> = IndexMap::new();

    for (i, key) in keys.iter().enumerate() {
        let index = i + 1;
        let id = format!("key{index}");
        let colid = format!("{id}col");
        let rowid = format!("{id}row");

        let label = first_non_empty_label(&key.labels);
        let mut row_net = id.clone();
        let mut col_net = "GND".to_string();
        if let Some((r, c)) = label.split_once('_')
            && !r.is_empty()
            && !c.is_empty()
            && r.chars().all(|ch| ch.is_ascii_digit())
            && c.chars().all(|ch| ch.is_ascii_digit())
        {
            row_net = format!("row_{r}");
            col_net = format!("col_{c}");
        }

        let x = key.x + (key.width - 1.0) / 2.0;
        let y = key.y + (key.height - 1.0) / 2.0;
        let origin_x = key.rotation_x - 0.5;
        let origin_y = key.rotation_y - 0.5;
        let x = norm_zero(x);
        let y = norm_zero(y);
        let origin_x = norm_zero(origin_x);
        let origin_y = norm_zero(origin_y);
        let splay = norm_zero(-key.rotation_angle);

        let mut row_meta = meta.clone();
        row_meta.insert("width".to_string(), Value::String(format!("{} u", key.width)));
        row_meta.insert("height".to_string(), Value::String(format!("{} u", key.height)));
        row_meta.insert("label".to_string(), Value::String(label));
        row_meta.insert("column_net".to_string(), Value::String(col_net));
        row_meta.insert("row_net".to_string(), Value::String(row_net));

        let mut rows: IndexMap<String, Value> = IndexMap::new();
        rows.insert(rowid, Value::Map(row_meta));

        let mut columns: IndexMap<String, Value> = IndexMap::new();
        columns.insert(
            colid,
            Value::Map(IndexMap::from([("rows".to_string(), Value::Map(rows))])),
        );

        let key_obj = Value::Map(IndexMap::from([
            (
                "origin".to_string(),
                Value::Seq(vec![
                    Value::String(format!("{} u", origin_x)),
                    Value::String(format!("{} u", norm_zero(-origin_y))),
                ]),
            ),
            ("splay".to_string(), Value::Number(splay)),
            (
                "shift".to_string(),
                Value::Seq(vec![
                    Value::String(format!("{} u", x)),
                    Value::String(format!("{} u", norm_zero(-y))),
                ]),
            ),
        ]));

        let zone = Value::Map(IndexMap::from([
            ("key".to_string(), key_obj),
            ("columns".to_string(), Value::Map(columns)),
        ]));

        zones.insert(id, zone);
    }

    Ok(Value::Map(IndexMap::from([(
        "points".to_string(),
        Value::Map(IndexMap::from([(
            "zones".to_string(),
            Value::Map(zones),
        )])),
    )])))
}

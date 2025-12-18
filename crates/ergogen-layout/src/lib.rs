use std::collections::HashMap;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ergogen_core::Point;
use ergogen_parser::Units;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnchorConfig {
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

pub struct LayoutContext<'a> {
    pub points: &'a HashMap<String, Point>,
    pub units: &'a Units,
}

pub fn resolve_anchor(config: &AnchorConfig, ctx: &LayoutContext) -> Result<Point> {
    match config {
        AnchorConfig::String(s) => resolve_string_anchor(s, ctx),
        AnchorConfig::Array(a) => resolve_array_anchor(a, ctx),
        AnchorConfig::Object(o) => resolve_object_anchor(o, ctx),
    }
}

fn resolve_string_anchor(s: &str, ctx: &LayoutContext) -> Result<Point> {
    if let Some(p) = ctx.points.get(s) {
        Ok(p.clone())
    } else {
        Err(anyhow!("Unknown point reference: {}", s))
    }
}

fn resolve_array_anchor(a: &[Value], ctx: &LayoutContext) -> Result<Point> {
    if a.len() == 2 {
        let x = ctx.units.parse(&a[0])?;
        let y = ctx.units.parse(&a[1])?;
        Ok(Point::new(x, y, 0.0))
    } else if a.len() == 3 {
        let x = ctx.units.parse(&a[0])?;
        let y = ctx.units.parse(&a[1])?;
        let r = ctx.units.parse(&a[2])?;
        Ok(Point::new(x, y, r))
    } else {
        Err(anyhow!("Invalid array anchor: {:?}", a))
    }
}

fn resolve_object_anchor(o: &HashMap<String, Value>, ctx: &LayoutContext) -> Result<Point> {
    let mut point = if let Some(ref_val) = o.get("ref") {
        let ref_config = serde_json::from_value(ref_val.clone())?;
        resolve_anchor(&ref_config, ctx)?
    } else {
        Point::new(0.0, 0.0, 0.0)
    };

    if let Some(shift_val) = o.get("shift") {
        if let Value::Array(shift_arr) = shift_val {
            if shift_arr.len() == 2 {
                let dx = ctx.units.parse(&shift_arr[0])?;
                let dy = ctx.units.parse(&shift_arr[1])?;
                point.shift([dx, dy]);
            }
        }
    }

    if let Some(rotate_val) = o.get("rotate") {
        let angle = ctx.units.parse(rotate_val)?;
        point.rotate(angle, None);
    }

    Ok(point)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsConfig {
    #[serde(default)]
    pub zones: HashMap<String, ZoneConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfig {
    #[serde(default)]
    pub anchor: Option<AnchorConfig>,
    #[serde(default)]
    pub columns: HashMap<String, ColumnConfig>,
    #[serde(default)]
    pub rows: HashMap<String, RowConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnConfig {
    #[serde(default)]
    pub rows: HashMap<String, RowConfig>,
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowConfig {
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

pub fn generate(config: &PointsConfig, units: &Units) -> Result<HashMap<String, Point>> {
    let mut points = HashMap::new();

    for (zone_name, zone) in &config.zones {
        let zone_anchor = if let Some(anchor_cfg) = &zone.anchor {
            resolve_anchor(anchor_cfg, &LayoutContext { points: &points, units })?
        } else {
            Point::new(0.0, 0.0, 0.0)
        };

        // Simplified layout logic: just iterate columns and rows for now
        for (col_name, _col) in &zone.columns {
            for (row_name, _row) in &zone.rows {
                let mut p = zone_anchor.clone();
                // In a real implementation, we'd apply stagger, spread, etc.
                // For MVP, just use a simple grid
                
                let name = format!("{}_{}_{}", zone_name, col_name, row_name);
                p.meta.name = name.clone();
                p.meta.zone = zone_name.clone();
                p.meta.column = col_name.clone();
                p.meta.row = row_name.clone();
                
                points.insert(name, p);
            }
        }
    }

    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_simple() {
        let units = Units::default();
        let config_json = json!({
            "zones": {
                "matrix": {
                    "columns": {
                        "index": {}
                    },
                    "rows": {
                        "home": {}
                    }
                }
            }
        });
        let config: PointsConfig = serde_json::from_value(config_json).unwrap();
        let points = generate(&config, &units).unwrap();
        
        assert!(points.contains_key("matrix_index_home"));
        let p = points.get("matrix_index_home").unwrap();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }
}

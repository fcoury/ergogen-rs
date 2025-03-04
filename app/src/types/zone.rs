use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{config::Config, Anchor, Key, Unit};
use crate::{types::points::apply_rotations, Result};

#[derive(Clone, Debug, Default)]
pub struct Point {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub r: Option<f64>,
    pub meta: Option<ParsedMeta>,
}

impl Point {
    pub fn p(&self) -> (f64, f64) {
        (self.x.unwrap_or_default(), self.y.unwrap_or_default())
    }

    pub fn set_p(&mut self, x: f64, y: f64) {
        self.x = Some(x);
        self.y = Some(y);
    }

    pub fn rotate(&mut self, angle: f64, origin: Option<(f64, f64)>, resist: bool) -> &mut Self {
        let mirrored = self.meta.as_ref().map_or(false, |meta| meta.mirrored);
        let angle = angle * if !resist && mirrored { -1.0 } else { 1.0 };
        if let Some(_origin) = origin {
            // TODO: this.set_p(makerjs.point.rotate(self, angle, origin));
            todo!("waiting for maker-rs");
        }
        self.r = Some(self.r.unwrap_or_default() + angle);
        self
    }

    pub fn angle(&self, other: &Point) -> f64 {
        let dx = other.x.unwrap_or_default() - self.x.unwrap_or_default();
        let dy = other.y.unwrap_or_default() - self.y.unwrap_or_default();

        -f64::atan2(dy, dx) * (180.0 / std::f64::consts::PI)
    }

    pub fn shift(
        &mut self,
        pos: (f64, f64),
        relative: Option<bool>,
        resist: Option<bool>,
    ) -> &mut Self {
        let (x, y) = pos;
        let relative = relative.unwrap_or(true);
        let resist = resist.unwrap_or(false);

        let x = if !resist && self.meta.as_ref().map_or(false, |meta| meta.mirrored) {
            -x
        } else {
            x
        };

        if relative {
            // TODO: (self.x, self.y) = makerjs.point.rotate([x, y], this.r);
            todo!("waiting for maker-rs");
        }

        self.x = Some(self.x.unwrap_or_default() + x);
        self.y = Some(self.y.unwrap_or_default() + y);
        self
    }
}

impl From<Point> for (f64, f64) {
    fn from(point: Point) -> Self {
        (point.x.unwrap_or_default(), point.y.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParsedMeta {
    stagger: f64,
    spread: f64,
    origin: [f64; 2],
    orient: f64,
    shift: (f64, f64),
    rotate: f64,
    // TODO: adjust: {}
    width: f64,
    height: f64,
    padding: f64,
    autobind: f64,
    skip: bool,
    asym: String,
    colrow: String,
    name: String,
    mirrored: bool,
    // zone: ParsedZone,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub name: Option<String>,
    pub anchor: Option<Anchor>,
    pub columns: Option<IndexMap<String, Column>>,
    pub rows: Option<IndexMap<String, Option<Row>>>,
    pub key: Option<Key>,
    pub mirror: Option<bool>,
    pub rotate: Option<Unit>,
}

impl Zone {
    pub fn columns(&self) -> IndexMap<String, Column> {
        self.columns.clone().unwrap_or_default()
    }

    pub fn rows(&self) -> IndexMap<String, Row> {
        let rows = self.rows.clone().unwrap_or_default();
        rows.into_iter()
            .map(|(name, row)| (name.clone(), row.unwrap_or(Row { name: Some(name) })))
            .collect()
    }

    pub fn render(
        &self,
        config: &Config,
        anchor: Point,
        units: &IndexMap<String, f64>,
    ) -> Result<IndexMap<String, Point>> {
        // let mut points = IndexMap::new();
        let mut rotations = Vec::new();
        let mut zone_anchor = anchor.clone();

        let mut first_col = true;
        for (col_name, col) in self.columns().iter() {
            println!("  - processing column {col_name}...");
            let mut col = col.clone();
            col.name = Some(col_name.clone());

            // combining row data from zone-wide defs and col-specific defs
            let mut actual_rows = self.rows();
            for (name, row) in col.rows().iter() {
                if let Some(row) = row {
                    actual_rows.insert(name.clone(), row.clone());
                }
            }

            // getting key config through the 5-level extension
            let mut keys = vec![];
            for (row_name, row) in actual_rows.iter_mut() {
                println!("    - processing row {row_name}...");
                row.name = Some(row_name.clone());
                let key = create_key(config, self, &col, col_name, row_name)?;
                keys.push(key);
            }

            if !first_col {
                // TODO: avoid the clone here, maybe the key can calculate its spread, taking unit?
                let spread = keys[0]
                    .clone()
                    .spread
                    .unwrap_or_default()
                    .eval_as_number("keys[0].spread", units)?;
                zone_anchor.x = Some(zone_anchor.x.unwrap_or_default() + spread);
            }
            // TODO: avoid the clone here, maybe the key can calculate its stagger, taking unit?
            let stagger = keys[0]
                .clone()
                .stagger
                .unwrap_or_default()
                .eval_as_number("keys[0].stagger", units)?;
            zone_anchor.y = Some(anchor.y.unwrap_or_default() + stagger);

            // applying col-level rotation (cumulatively, for the next columns as well)
            let col_anchor = zone_anchor.clone();
            if let Some(splay) = &keys[0].splay {
                let splay = splay.eval_as_number("keys[0].splay", units)?;
                // TODO: avoid the clone here if possible on a refactor
                let current_rotations = rotations.clone();
                let new_rotation = apply_rotations(&current_rotations, splay, &col_anchor);
                rotations.push(new_rotation);
            }

            // actually laying out keys
            let mut running_anchor = col_anchor.clone();
            for r in &rotations {
                // TODO: avoid the clone here if possible on a refactor
                running_anchor.rotate(r.angle, Some(r.origin.clone().into()), false);
            }

            for key in keys {
                // copy the current column anchor
                let meta = running_anchor.meta.clone().unwrap_or_default();
                let mut point = running_anchor.clone();

                // apply cumulative per-key adjustments
                if let Some(orient) = key.orient {
                    let orient = orient.eval_as_number("key.orient", units)?;
                    point.r = Some(point.r.unwrap_or_default() + orient);
                }

                point.shift(meta.shift, None, None);

                if let Some(rotate) = key.rotate {
                    let rotate = rotate.eval_as_number("key.rotate", units)?;
                    point.r = Some(point.r.unwrap_or_default() + rotate);
                }

                // commit running anchor
                running_anchor = point.clone();

                // apply independent adjustments
                // point =
            }
        }

        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    name: Option<String>,
    key: Option<Key>,
    rows: Option<IndexMap<String, Option<Row>>>,
}

impl Column {
    pub fn rows(&self) -> IndexMap<String, Option<Row>> {
        self.rows.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {
    name: Option<String>,
}

pub fn create_key(
    config: &Config,
    zone: &Zone,
    col: &Column,
    col_name: impl ToString,
    row_name: impl ToString,
) -> Result<Key> {
    let mut key = Key::new_default(&config.units());

    key.zone = Some(Box::new(zone.clone()));
    key.row = Some(row_name.to_string());
    key.col_name = Some(col_name.to_string());
    key.col = Some(Box::new(col.clone()));

    // layer the keys
    if let Some(global_key) = &config.points.key {
        // global key
        key.extend(global_key);
    }

    if let Some(zone_wide_key) = &zone.key {
        // zone-wide key
        key.extend(zone_wide_key);
    }

    if let Some(col_key) = &col.key {
        // column key
        key.extend(col_key);
    }

    key.process_templates()
}

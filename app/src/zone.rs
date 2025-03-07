use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    config::Config,
    types::{Asym, Bind, Key, Mirror, Unit},
};
use crate::{anchor::Anchor, points::apply_rotations, Result};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub r: Option<f64>,
    pub meta: Option<ParsedMeta>,
}

impl Point {
    pub fn is_skip(&self) -> bool {
        self.meta.as_ref().is_some_and(|meta| meta.skip)
    }

    pub fn p(&self) -> (f64, f64) {
        (self.x.unwrap_or_default(), self.y.unwrap_or_default())
    }

    pub fn set_p(&mut self, p: (f64, f64)) {
        self.x = Some(p.0);
        self.y = Some(p.1);
    }

    pub fn rotate(&mut self, angle: f64, origin: Option<(f64, f64)>, resist: bool) -> &mut Self {
        let mirrored = self.meta.as_ref().is_some_and(|meta| meta.mirrored);
        let angle = angle * if !resist && mirrored { -1.0 } else { 1.0 };
        if let Some(origin) = origin {
            self.set_p(maker_rs::point::rotate(
                self.clone().into(),
                angle,
                Some(origin),
            ));
        }
        self.r = Some(self.r.unwrap_or_default() + angle);
        self
    }

    pub fn rotated(&self, angle: f64, origin: Option<(f64, f64)>, resist: bool) -> Self {
        let mut point = self.clone();
        point.rotate(angle, origin, resist);
        point
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

        let x = if !resist && self.meta.as_ref().is_some_and(|meta| meta.mirrored) {
            -x
        } else {
            x
        };

        if relative {
            let (x, y) = maker_rs::point::rotate((x, y), self.r.unwrap_or_default(), None);
            (self.x, self.y) = (Some(x), Some(y));
        }

        self.x = Some(self.x.unwrap_or_default() + x);
        self.y = Some(self.y.unwrap_or_default() + y);
        self
    }

    pub fn mirror(&mut self, x_axis: f64) -> &mut Self {
        let x = self.x.unwrap_or_default();
        self.x = Some(2.0 * x_axis - x);
        self
    }

    pub fn mirrored(&self, x_axis: f64) -> Self {
        let x = self.x.unwrap_or_default();
        let x = 2.0 * x_axis - x;

        Point {
            x: Some(x),
            y: self.y,
            r: self.r,
            meta: self.meta.clone(),
        }
    }

    pub fn meta_bind(&self, units: &IndexMap<String, f64>) -> Result<Option<[f64; 4]>> {
        let option_result = self
            .meta
            .as_ref()
            .and_then(|meta| meta.bind.as_ref())
            .map(|bind| bind.resolve(units));

        match option_result {
            Some(result) => result.map(Some),
            None => Ok(None),
        }
    }

    pub fn meta_col_name(&self) -> &str {
        self.meta.as_ref().map_or("", |meta| meta.colrow.as_str())
    }

    pub fn meta_zone_columns(&self) -> IndexMap<String, Column> {
        self.meta.as_ref().map_or(IndexMap::new(), |meta| {
            meta.zone
                .as_ref()
                .map_or(IndexMap::new(), |zone| zone.columns())
        })
    }
}

pub fn perform_mirror(point: &Point, axis: f64) -> (String, Option<Point>) {
    let Some(meta) = &point.meta else {
        return ("".to_string(), None);
    };

    let mut meta = meta.clone();
    meta.mirrored = false;

    if let Some(asym) = meta.asym {
        if asym.is_source() {
            return ("".to_string(), None);
        }
    }

    let mut mirrored_point = point.mirrored(axis);

    let mirrored_name = format!("mirror_{}", meta.colrow);
    let mut new_meta = meta.clone();
    new_meta.colrow = mirrored_name.clone();
    new_meta.mirrored = true;
    if let Some(asym) = new_meta.asym {
        if asym.is_clone() {
            new_meta.skip = true;
        }
    }

    // TODO: we're missing this mp.meta = prep.extend(mp.meta, mp.meta.mirror || {});
    mirrored_point.meta = Some(new_meta);

    (mirrored_name, Some(mirrored_point.clone()))
}

impl From<Point> for (f64, f64) {
    fn from(point: Point) -> Self {
        (point.x.unwrap_or_default(), point.y.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParsedMeta {
    pub stagger: f64,
    pub spread: f64,
    pub origin: (f64, f64),
    pub orient: f64,
    pub shift: (f64, f64),
    pub rotate: f64,
    // TODO: adjust: {}
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub autobind: f64,
    pub skip: bool,
    pub asym: Option<Asym>,
    pub colrow: String,
    pub name: String,
    pub mirrored: bool,
    // zone: ParsedZone,
    pub zone: Option<Zone>,
    pub bind: Option<Bind>,
}

impl ParsedMeta {
    pub fn zone_name(&self) -> String {
        self.zone
            .as_ref()
            .map_or("".to_string(), |zone| zone.name.clone().unwrap_or_default())
    }

    fn from(zone: &Zone, key: Key, units: &IndexMap<String, f64>) -> Result<Self> {
        let mut meta = ParsedMeta::default();
        let key_name = key.name.unwrap_or("key".to_string());

        if let Some(stagger) = key.stagger {
            meta.stagger = stagger.eval_as_number(&format!("{key_name}.stagger"), units)?;
        }

        if let Some(spread) = key.spread {
            meta.spread = spread.eval_as_number(&format!("{key_name}.spread"), units)?;
        }

        if let Some(origin) = key.origin {
            let (x, y) = origin;
            let x = x.eval_as_number("key.origin.x", units)?;
            let y = y.eval_as_number("key.origin.y", units)?;
            meta.origin = (x, y);
        }

        if let Some(orient) = key.orient {
            meta.orient = orient.eval_as_number(&format!("{key_name}.orient"), units)?;
        }

        if let Some(shift) = key.shift {
            meta.shift = shift.eval_as_numbers(&format!("{key_name}.shift"), units)?;
        }

        if let Some(rotate) = key.rotate {
            meta.rotate = rotate.eval_as_number(&format!("{key_name}.rotate"), units)?;
        }

        if let Some(width) = key.width {
            meta.width = width.eval_as_number(&format!("{key_name}.rotate"), units)?;
        }

        if let Some(height) = key.height {
            meta.height = height.eval_as_number(&format!("{key_name}.height"), units)?;
        }

        if let Some(padding) = key.padding {
            meta.padding = padding.eval_as_number(&format!("{key_name}.padding"), units)?;
        }

        if let Some(autobind) = key.autobind {
            meta.autobind = autobind.eval_as_number(&format!("{key_name}.autobind"), units)?;
        }

        if let Some(skip) = key.skip {
            meta.skip = skip;
        }

        meta.asym = key.asym;

        if let Some(colrow) = key.colrow {
            meta.colrow = colrow;
        }

        meta.name = key_name;
        meta.zone = Some(zone.clone());

        // TODO: How to handle mirroring here?
        // if let Some(mirrored) = key.mirror {
        //     meta.mirrored = mirrored;
        // }

        Ok(meta)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<IndexMap<String, Column>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<IndexMap<String, Option<Row>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror: Option<Mirror>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
        let mut points = IndexMap::new();
        let mut rotations = Vec::new();
        let mut zone_anchor = anchor.clone();

        let mut first_col = true;
        let mut columns = self.columns();
        if columns.is_empty() {
            columns.insert("default".to_string(), Column::default());
        }
        for (col_name, col) in columns.iter() {
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
            if actual_rows.is_empty() {
                actual_rows.insert(
                    "default".to_string(),
                    Row {
                        name: Some("".to_string()),
                    },
                );
            }

            // getting key config through the 5-level extension
            let mut keys = vec![];
            for (row_name, row) in actual_rows.iter_mut() {
                println!("    - processing row {row_name}...");
                row.name = Some(row_name.clone());
                let key = create_key(config, self, &col, col_name, row_name)?;
                keys.push(key);
            }

            let default_key = Key::default();
            let first_key = keys.first().unwrap_or(&default_key);

            if !first_col {
                // TODO: avoid the clone here, maybe the key can calculate its spread, taking unit?
                let spread = first_key
                    .clone()
                    .spread
                    .unwrap_or_default()
                    .eval_as_number("first_key.spread", units)?;
                zone_anchor.x = Some(zone_anchor.x.unwrap_or_default() + spread);
            }
            // TODO: avoid the clone here, maybe the key can calculate its stagger, taking unit?
            let stagger = first_key
                .clone()
                .stagger
                .unwrap_or_default()
                .eval_as_number("first_key.stagger", units)?;
            zone_anchor.y = Some(anchor.y.unwrap_or_default() + stagger);

            // applying col-level rotation (cumulatively, for the next columns as well)
            let col_anchor = zone_anchor.clone();
            if let Some(splay) = &first_key.splay {
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
                let key_name = key.name.clone().unwrap_or_default();

                // copy the current column anchor
                let meta = running_anchor.meta.clone().unwrap_or_default();
                let mut point = running_anchor.clone();

                // apply cumulative per-key adjustments
                if let Some(orient) = &key.orient {
                    let orient = orient.eval_as_number(&format!("{key_name}.orient"), units)?;
                    point.r = Some(point.r.unwrap_or_default() + orient);
                }

                point.shift(meta.shift, None, None);

                if let Some(rotate) = &key.rotate {
                    let rotate = rotate.eval_as_number(&format!("{key_name}.rotate"), units)?;
                    point.r = Some(point.r.unwrap_or_default() + rotate);
                }

                // commit running anchor
                running_anchor = point.clone();

                // apply independent adjustments
                if let Some(adjust) = &key.adjust {
                    point = adjust.parse(
                        format!("{key_name}.adjust"),
                        &IndexMap::new(),
                        None,
                        false,
                        units,
                    )?;
                }

                // save the key
                let padding = key
                    .padding
                    .clone()
                    .unwrap_or_default()
                    .eval_as_number("key.padding", units)?;
                point.meta = Some(ParsedMeta::from(self, key, units)?);
                points.insert(key_name, point);

                // advance the running anchor to the next position
                running_anchor.shift((0.0, padding), None, None);
            }

            first_col = false;
        }

        println!("total points: {}", points.len());
        Ok(points)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Column {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<IndexMap<String, Option<Row>>>,
}

impl Column {
    pub fn rows(&self) -> IndexMap<String, Option<Row>> {
        self.rows.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {
    #[serde(skip_serializing_if = "Option::is_none")]
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

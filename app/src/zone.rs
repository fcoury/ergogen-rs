use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    config::Config,
    types::{Key, Mirror, Unit},
};
use crate::{
    anchor::{parse_anchored, AffectType, Anchor},
    point::{AnchorInfo, Point},
    points::{apply_rotations, Rotation},
    template::process_templates,
    types::Asym,
    Error, Result,
};

pub fn perform_mirror(point: &Point, axis: f64) -> (String, Option<Point>) {
    let Some(meta) = &point.meta else {
        return ("".to_string(), None);
    };

    let mut meta = meta.clone();
    meta.mirrored = Some(false);

    if let Some(asym) = meta.asym {
        if asym.is_source() {
            return ("".to_string(), None);
        }
    }

    let mut mirrored_point = point.mirrored(axis);

    let mirrored_name = format!("mirror_{}", meta.colrow.clone().unwrap_or_default());
    let mut new_meta = meta.clone();
    new_meta.colrow = Some(mirrored_name.clone());
    new_meta.mirrored = Some(true);
    if let Some(asym) = new_meta.asym {
        if asym.is_clone() {
            new_meta.skip = Some(true);
        }
    }

    // TODO: we're missing this mp.meta = prep.extend(mp.meta, mp.meta.mirror || {});
    mirrored_point.meta = Some(new_meta);

    (mirrored_name, Some(mirrored_point.clone()))
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
    pub rows: Option<IndexMap<String, Row>>,
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

    pub fn render(
        &self,
        config: &Config,
        anchor: Point,
        units: &IndexMap<String, f64>,
    ) -> Result<IndexMap<String, Point>> {
        let mut points = IndexMap::new();
        let mut rotations = Vec::new();
        let mut zone_anchor = anchor.clone();

        // transferring the anchor rotation to "real" rotations
        rotations.push(Rotation {
            angle: zone_anchor.r.unwrap_or_default(),
            origin: zone_anchor.p(),
        });
        // and now clear it from the anchor so that we don't apply it twice
        zone_anchor.r = None;

        let mut first_col = true;
        let mut columns = self.columns();
        if columns.is_empty() {
            columns.insert("default".to_string(), Column::default());
        }
        for (col_name, col) in columns.iter() {
            println!("  - Processing column: {col_name}...");
            let mut col = col.clone();
            col.name = Some(col_name.clone());

            // combining row data from zone-wide defs and col-specific defs
            let mut actual_rows = self.row_names();
            actual_rows.extend(col.row_names());

            for row in col.row_names().iter() {
                actual_rows.push(row);
            }
            if actual_rows.is_empty() {
                actual_rows.push("default");
            }

            // getting key config through the 5-level extension
            let mut keys = vec![];
            for row_name in actual_rows.iter_mut() {
                println!("    - Processing row: {row_name}...");
                let key = create_key(config, self, &col, col_name, row_name, units)?;
                println!("      - adding key: {:?}", key.name);
                keys.push(key);
            }

            let default_key = ParsedKey::new_default(units);
            let first_key = keys.first().unwrap_or(&default_key);

            if !first_col {
                // TODO: avoid the clone here, maybe the key can calculate its spread, taking unit?
                let spread = first_key.spread.unwrap_or_default();
                zone_anchor.x = Some(zone_anchor.x.unwrap_or_default() + spread);
            }
            // TODO: avoid the clone here, maybe the key can calculate its stagger, taking unit?
            let stagger = first_key.clone().stagger.unwrap_or_default();
            zone_anchor.y = Some(anchor.y.unwrap_or_default() + stagger);

            // applying col-level rotation (cumulatively, for the next columns as well)
            let col_anchor = zone_anchor.clone();
            if let Some(splay) = &first_key.splay {
                if splay != &0.0 {
                    // TODO: avoid the clone here if possible on a refactor
                    let current_rotations = rotations.clone();
                    let origin = keys[0].origin.unwrap_or_default();
                    let mut col_anchor = col_anchor.clone();
                    col_anchor.shift(origin, Some(false), None);
                    let anchor: (f64, f64) = col_anchor.clone().into();
                    let new_rotation = apply_rotations(&current_rotations, *splay, anchor);
                    rotations.push(new_rotation);
                }
            }
            println!("Rotations: {:#?}", rotations);

            // actually laying out keys
            let mut running_anchor = col_anchor.clone();
            println!("Running anchor: {:#?}", running_anchor);
            for r in &rotations {
                running_anchor.rotate(r.angle, Some(r.origin), false);
            }
            println!("Running anchor after rotation: {:#?}", running_anchor);

            for key in keys {
                let key_name = key.name.clone().unwrap_or_default();

                // copy the current column anchor
                // let meta = running_anchor.meta.clone().unwrap_or_default();
                let mut point = running_anchor.clone();

                // apply cumulative per-key adjustments
                if let Some(orient) = &key.orient {
                    point.r = Some(point.r.unwrap_or_default() + orient);
                }

                let shift = key.shift.unwrap_or_default();
                point.shift(shift, None, None);

                if let Some(rotate) = &key.rotate {
                    point.r = Some(point.r.unwrap_or_default() + rotate);
                }

                // commit running anchor
                running_anchor = point.clone();

                // apply independent adjustments
                if let Some(adjust) = &key.adjust {
                    point = parse_anchored(
                        adjust,
                        format!("{key_name}.adjust"),
                        &IndexMap::new(),
                        Some(point),
                        false,
                        units,
                    )?;
                }

                // save the key
                let padding = key.padding.unwrap_or_default();
                point.meta = Some(key.into());
                points.insert(key_name, point);

                // advance the running anchor to the next position
                running_anchor.shift((0.0, padding), None, None);
            }

            first_col = false;
        }

        Ok(points)
    }

    fn row_names(&self) -> Vec<&str> {
        self.rows
            .as_ref()
            .map_or(vec![], |rows| rows.keys().map(|k| k.as_str()).collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Row {
    #[serde(flatten)]
    pub anchor: Option<AnchorInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Column {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<IndexMap<String, Row>>,
}

impl Column {
    fn row_names(&self) -> Vec<&str> {
        self.rows
            .as_ref()
            .map_or(vec![], |rows| rows.keys().map(|k| k.as_str()).collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ParsedKey {
    pub name: Option<String>,
    pub zone: Option<Zone>,
    pub row: Option<String>,
    pub col: Option<Column>,
    pub col_name: Option<String>,
    pub stagger: Option<f64>,
    pub spread: Option<f64>,
    pub splay: Option<f64>,
    pub origin: Option<(f64, f64)>,
    pub orient: Option<f64>,
    pub shift: Option<(f64, f64)>,
    pub rotate: Option<f64>,
    pub adjust: Option<AnchorInfo>,
    pub bind: Option<[f64; 4]>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub padding: Option<f64>,
    pub autobind: Option<f64>,
    pub skip: Option<bool>,
    pub asym: Option<Asym>,
    pub colrow: Option<String>,
    pub mirrored: Option<bool>,
    pub affect: Option<Vec<AffectType>>,
    pub meta: Option<IndexMap<String, serde_json::Value>>,
}

impl ParsedKey {
    pub fn new_default(units: &IndexMap<String, f64>) -> Self {
        ParsedKey {
            stagger: Some(*units.get("$default_stagger").unwrap()),
            spread: Some(*units.get("$default_spread").unwrap()),
            splay: Some(*units.get("$default_splay").unwrap()),
            origin: Some((0.0, 0.0)),
            orient: Some(0.0),
            shift: Some((0.0, 0.0)),
            rotate: Some(0.0),
            adjust: None,
            width: Some(*units.get("$default_width").unwrap()),
            height: Some(*units.get("$default_height").unwrap()),
            padding: Some(*units.get("$default_padding").unwrap()),
            autobind: Some(*units.get("$default_autobind").unwrap()),
            skip: Some(false),
            colrow: Some("{{col.name}}_{{row}}".to_owned()),
            name: Some("{{zone.name}}_{{colrow}}".to_owned()),
            ..Default::default()
        }
    }

    pub fn extend(&mut self, other: &ParsedKey) {
        if let Some(stagger) = &other.stagger {
            self.stagger = Some(*stagger);
        }
        if let Some(spread) = &other.spread {
            self.spread = Some(*spread);
        }
        if let Some(splay) = &other.splay {
            self.splay = Some(*splay);
        }
        if let Some(padding) = &other.padding {
            self.padding = Some(*padding);
        }
        if let Some(origin) = &other.origin {
            self.origin = Some(*origin);
        }
        if let Some(orient) = &other.orient {
            self.orient = Some(*orient);
        }
        if let Some(shift) = &other.shift {
            self.shift = Some(*shift);
        }
        if let Some(rotate) = &other.rotate {
            self.rotate = Some(*rotate);
        }
        if let Some(adjust) = &other.adjust {
            self.adjust = Some(adjust.clone());
        }
        if let Some(bind) = &other.bind {
            self.bind = Some(*bind);
        }
        if let Some(autobind) = &other.autobind {
            self.autobind = Some(*autobind);
        }
        if let Some(skip) = &other.skip {
            self.skip = Some(*skip);
        }
        if let Some(asym) = &other.asym {
            self.asym = Some(*asym);
        }
        // TODO: add mirror?
        // if let Some(mirror) = &other.mirror {
        //     self.mirror = Some(mirror.clone());
        // }
        if let Some(colrow) = &other.colrow {
            self.colrow = Some(colrow.clone());
        }
        if let Some(name) = &other.name {
            self.name = Some(name.clone());
        }
        if let Some(width) = &other.width {
            self.width = Some(*width);
        }
        if let Some(height) = &other.height {
            self.height = Some(*height);
        }

        // Handle the meta field specially
        if let Some(other_meta) = &other.meta {
            if self.meta.is_none() {
                // If self has no meta, just clone other's meta
                self.meta = Some(other_meta.clone());
            } else if let Some(self_meta) = &mut self.meta {
                // If both have meta, extend self's meta with other's meta
                for (key, value) in other_meta {
                    self_meta.insert(key.clone(), value.clone());
                }
            }
        }
    }

    pub fn process_templates(&self) -> Result<ParsedKey> {
        let key_obj = serde_json::to_value(self)?;
        let key_obj = key_obj.as_object().ok_or(Error::TypeError {
            field: "key".to_owned(),
            expected: "object".to_owned(),
        })?;

        let key_obj = process_templates(key_obj);

        Ok(serde_json::from_value(Value::Object(key_obj))?)
    }
}

pub fn create_key(
    config: &Config,
    zone: &Zone,
    col: &Column,
    col_name: impl ToString,
    row_name: impl ToString,
    units: &IndexMap<String, f64>,
) -> Result<ParsedKey> {
    let mut key = ParsedKey::new_default(units);

    key.zone = Some(zone.clone());
    key.row = Some(row_name.to_string());
    key.col = Some(col.clone());
    key.col_name = Some(col_name.to_string());

    // layer the keys
    if let Some(global_key) = &config.points.key {
        // global key
        let global_key = global_key.resolve(units)?;
        key.extend(&global_key);
    }

    if let Some(zone_wide_key) = &zone.key {
        // zone-wide key
        let zone_wide_key = zone_wide_key.resolve(units)?;
        key.extend(&zone_wide_key);
    }

    if let Some(col_key) = &col.key {
        // column key
        let col_key = col_key.resolve(units)?;
        key.extend(&col_key);
    }

    key.process_templates()
}

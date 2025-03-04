use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{config::Config, template::process_templates, Anchor, Key, Units};
use crate::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct Point {
    x: Option<f64>,
    y: Option<f64>,
    r: Option<f64>,
    meta: ParsedMeta,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedMeta {
    stagger: f64,
    spread: f64,
    origin: [f64; 2],
    orient: f64,
    shift: [f64; 2],
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
    // zone: ParsedZone,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub name: Option<String>,
    pub anchor: Option<Anchor>,
    pub columns: Option<IndexMap<String, Column>>,
    pub rows: Option<IndexMap<String, Option<Row>>>,
    pub key: Option<Key>,
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

    pub fn render(&self, config: &Config, anchor: Option<&Anchor>) -> Result<Point> {
        let zone_anchor = anchor.clone();

        for (col_name, col) in self.columns().iter() {
            println!("  - processing column {col_name}...");
            let mut col = col.clone();
            col.name = Some(col_name.clone());

            // expand the zone wide rows with this column specific ones
            let mut actual_rows = self.rows();
            for (name, row) in col.rows().iter() {
                if let Some(row) = row {
                    actual_rows.insert(name.clone(), row.clone());
                }
            }

            let mut keys = vec![];
            for (row_name, row) in actual_rows.iter_mut() {
                println!("    - processing row {row_name}...");
                row.name = Some(row_name.clone());
                let key = create_key(config, self, &col, col_name, row_name)?;

                keys.push(key);
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

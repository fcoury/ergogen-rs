use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{config::Config, Anchor, Key, Units};
use crate::Result;

pub struct Point {
    x: Option<f64>,
    y: Option<f64>,
    r: Option<f64>,
    meta: ParsedMeta,
}

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
    anchor: Option<Anchor>,
    columns: Option<IndexMap<String, Column>>,
    rows: Option<IndexMap<String, Option<Row>>>,
    key: Option<Key>,
}

impl Zone {
    pub fn columns(&self) -> IndexMap<String, Column> {
        self.columns.clone().unwrap_or_default()
    }

    pub fn rows(&self) -> IndexMap<String, Row> {
        let rows = self.rows.clone().unwrap_or_default();
        rows.into_iter()
            .map(|(k, v)| (k, v.unwrap_or_default()))
            .collect()
    }

    pub fn anchor(&self, units: &Units) -> Anchor {
        self.anchor.clone().unwrap_or_default().parse(units)
    }

    pub fn render(&self, config: &Config) -> Result<Point> {
        for (name, col) in self.columns().iter() {
            println!("  - processing column {name}...");

            // expand the zone wide rows with this column specific ones
            let mut actual_rows = self.rows();
            for (name, row) in col.rows().iter() {
                if let Some(row) = row {
                    actual_rows.insert(name.clone(), row.clone());
                }
            }

            for (name, row) in actual_rows.iter() {
                println!("    - processing row {name}...");
                row.process(config, self, col);
            }
            // println!("{:#?}", col);
        }

        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    key: Option<Key>,
    rows: Option<IndexMap<String, Option<Row>>>,
}

impl Column {
    pub fn rows(&self) -> IndexMap<String, Option<Row>> {
        self.rows.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {}

impl Default for Row {
    fn default() -> Self {
        Self {}
    }
}

impl Row {
    pub fn process(&self, config: &Config, zone: &Zone, col: &Column) {
        let default_key = Key::new_default(&config.units());
        let global_key = config.points.key.clone().unwrap_or_default();
        let zone_wide_key = zone.key.clone().unwrap_or_default();
        let col_key = col.key.clone().unwrap_or_default();
        todo!("Row::process")
    }
}

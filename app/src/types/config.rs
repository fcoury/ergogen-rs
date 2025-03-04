use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{preprocess::preprocess, Meta, Points, Unit, Units};
use crate::{types::zone::Point, units::evaluate_mathnum, Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub meta: Option<Meta>,
    pub units: Option<Units>,
    pub variables: Option<Units>,
    pub points: Points,
}

impl Config {
    pub fn parse(config: impl ToString) -> Result<Self> {
        let config = config.to_string();
        let config = serde_yaml::from_str(&config)?;
        let config = preprocess(config)?;

        Ok(serde_yaml::from_value(config)?)
    }

    pub fn resolve_units(&self) -> Result<IndexMap<String, f64>> {
        // Create a default units map
        let mut raw_units = IndexMap::<String, Unit>::from([
            ("U".to_string(), Unit::Number(19.05)),
            ("u".to_string(), Unit::Number(19.0)),
            ("cx".to_string(), Unit::Number(18.0)),
            ("cy".to_string(), Unit::Number(17.0)),
            ("$default_stagger".to_string(), Unit::Number(0.0)),
            (
                "$default_spread".to_string(),
                Unit::Expression("u".to_string()),
            ),
            ("$default_splay".to_string(), Unit::Number(0.0)),
            (
                "$default_height".to_string(),
                Unit::Expression("u-1".to_string()),
            ),
            (
                "$default_width".to_string(),
                Unit::Expression("u-1".to_string()),
            ),
            (
                "$default_padding".to_string(),
                Unit::Expression("u".to_string()),
            ),
            ("$default_autobind".to_string(), Unit::Number(10.0)),
        ]);

        // Extend with units from config
        if let Some(config_units) = &self.units {
            for (key, val) in config_units {
                raw_units.insert(key.clone(), val.clone());
            }
        }

        // Extend with variables from config
        if let Some(config_vars) = &self.variables {
            for (key, val) in config_vars {
                raw_units.insert(key.clone(), val.clone());
            }
        }

        // Calculate final units
        let mut units = IndexMap::<String, f64>::new();

        // Iterate fixed values
        let (fixed, calculated): (Vec<_>, Vec<_>) =
            raw_units.iter().partition(|(_, v)| v.is_number());

        for (key, val) in fixed {
            if let Some(f) = val.as_number() {
                units.insert(key.clone(), f);
            }
        }

        let mut last_failed_keys = Vec::new();
        loop {
            let mut failed_keys = Vec::new();

            for (key, val) in calculated.iter() {
                if failed_keys.contains(key) {
                    continue;
                }

                match evaluate_mathnum(val, &units) {
                    Ok(f) => {
                        units.insert(key.to_string(), f);
                    }
                    Err(e) => {
                        tracing::error!("Failed to evaluate unit '{}': {}", key, e);
                        failed_keys.push(key);
                    }
                }
            }

            if failed_keys.is_empty() {
                break;
            } else if last_failed_keys == failed_keys {
                return Err(Error::ValueError(format!(
                    "Failed to evaluate units: {:?}",
                    failed_keys
                )));
            }

            last_failed_keys = failed_keys.clone();
        }

        Ok(units)
    }

    pub fn parse_points(&self) -> Result<Points> {
        let mut points = IndexMap::new();
        let units = self.resolve_units()?;
        for (name, zone) in self.points.zones.iter() {
            println!("Processing zone {name}...");
            let mut zone = zone.clone();
            zone.name = Some(name.to_string());

            // extracting keys that are handled here, not at the zone render level
            let anchor = match zone.anchor {
                Some(ref anchor) => {
                    let name = format!("points.zones.{name}.anchor");
                    println!("  - parsing anchor {name}...");
                    anchor.parse(name, &points, None, false, &units)?
                }
                None => Point::default(),
            };

            let mirror = zone.mirror.unwrap_or_default();
            zone.anchor = None;
            zone.rotate = None;
            zone.mirror = None;

            // creating new points
            let new_points = zone.render(self, anchor, &units)?;
            println!("{:#?}", new_points);

            // simplifying the names in individual point "zones" and single-key columns
        }

        todo!()
    }

    pub fn units(&self) -> Units {
        self.units.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn test_parse() {
        for file in fs::read_dir("tests/points").unwrap() {
            let Ok(file) = file else {
                continue;
            };

            let path = file.path();
            let ext = path.extension().unwrap_or_default();

            if ext != "yaml" {
                continue;
            }

            if path.file_name().unwrap() == "overrides.yaml" {
                // TODO: $unset is not implemented
                continue;
            }

            println!("Parsing {:?}", file.path());
            let contents = fs::read_to_string(file.path()).unwrap();
            let config = Config::parse(contents).unwrap();

            let file = file.path();
            let parent = file.parent().unwrap_or(Path::new(""));
            let file_stem = file.file_stem().unwrap().to_str().unwrap();
            let points_file = parent.join(format!("{file_stem}___points.json"));

            if !points_file.exists() {
                continue;
            }

            println!("reading {:?}", points_file);
            let expected = fs::read_to_string(points_file).unwrap();
            let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();

            let actual = config.parse_points().unwrap();

            println!("{:#?}", expected);
            println!("{:#?}", actual);
        }
    }

    #[test]
    fn parse_basic() {
        let config = include_str!("../../tests/points/basic_2x2.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn parse_with_adjustments() {
        let config = include_str!("../../tests/points/adjustments.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn parse_autobind() {
        let config = include_str!("../../tests/points/autobind.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_default() {
        let config = include_str!("../../tests/points/default.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_mirrors() {
        let config = include_str!("../../tests/points/mirrors.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_rotations() {
        let config = include_str!("../../tests/points/rotations.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_samename() {
        let config = include_str!("../../tests/points/samename.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_units() {
        let config = include_str!("../../tests/points/units.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    #[ignore = "$unset is not implemented"]
    fn test_parse_unset() {
        let config = include_str!("../../tests/points/overrides.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }
}

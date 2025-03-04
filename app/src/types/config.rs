use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{preprocess::preprocess, Meta, Points, Units};
use crate::Result;

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

    pub fn parse_points(&self) -> Result<Points> {
        let mut points = IndexMap::new();
        for (name, zone) in self.points.zones.iter() {
            println!("Processing zone {name}...");
            let mut zone = zone.clone();
            zone.name = Some(name.to_string());

            let anchor = match zone.anchor {
                Some(ref anchor) => {
                    let name = format!("points.zones.{name}.anchor");
                    println!("  - parsing anchor {name}...");
                    Some(anchor.parse(name, &points, None, false, &self.units())?)
                }
                None => None,
            };

            let new_points = zone.render(self, zone.anchor.as_ref())?;
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

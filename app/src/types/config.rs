use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{preprocess::preprocess, Meta, Mirror, Points, Unit, Units};
use crate::{types::zone::Point, units::evaluate_mathnum, Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub meta: Option<Meta>,
    pub units: Option<Units>,
    pub variables: Option<Units>,
    pub points: Points,
    pub rotate: Option<Unit>,
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

    pub fn parse_points(&self) -> Result<IndexMap<String, Point>> {
        // const global_rotate = a.sane(config.rotate || 0, 'points.rotate', 'number')(units)
        let global_rotate = match &self.rotate {
            Some(rotate) => {
                let rotate = rotate.eval_as_number("points.rotate", &self.resolve_units()?)?;
                Some(rotate)
            }
            None => None,
        };
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

            let mirror = zone.mirror;
            zone.anchor = None;
            zone.rotate = None;
            zone.mirror = None;

            // creating new points
            let new_points = zone.render(self, anchor, &units)?;

            // simplifying the names in individual point "zones" and single-key columns
            let mut renamed_points = IndexMap::new();
            for (name, point) in new_points.into_iter() {
                let name = name.strip_suffix("_default").unwrap_or(&name);
                renamed_points.insert(name.to_string(), point);
            }

            // adjusting new points
            for (new_name, new_point) in renamed_points.iter_mut() {
                if points.contains_key(new_name) {
                    return Err(Error::Config(format!(
                        "Key \"{new_name}\" defined more than once!",
                    )));
                }

                if let Some(ref rotate) = zone.rotate {
                    let rotate = rotate.eval_as_number(
                        &format!(
                            "zone \"{}\" rotation",
                            zone.name.clone().unwrap_or_default()
                        ),
                        &units,
                    )?;
                    new_point.rotate(rotate, None, false);
                }
            }

            // adding new points so that they can be referenced from now on
            points.extend(renamed_points);

            // TODO: per-zone mirroring for the new keys
            // let axis = self.parse_axis(
            //     mirror,
            //     format!("points.zones.{name}.mirror"),
            //     &points,
            //     &units,
            // )?;
            //
            // if (axis !== undefined) {
            //   const mirrored_points = {}
            //   for (const new_point of Object.values(new_points)) {
            //     const [mname, mp] = perform_mirror(new_point, axis)
            //     if (mp) {
            //       mirrored_points[mname] = mp
            //     }
            //   }
            //   points = Object.assign(points, mirrored_points)
            // }
        }

        // applying global rotation
        if let Some(global_rotate) = global_rotate {
            points = points
                .iter()
                .map(|(name, p)| (name.clone(), p.rotated(global_rotate, None, false)))
                .collect();
        }

        // global mirroring for points that haven't been mirrored yet
        // const global_axis = parse_axis(global_mirror, `points.mirror`, points, units)
        // const global_mirrored_points = {}
        // for (const point of Object.values(points)) {
        //   if (global_axis !== undefined && point.meta.mirrored === undefined) {
        //     const [mname, mp] = perform_mirror(point, global_axis)
        //     if (mp) {
        //       global_mirrored_points[mname] = mp
        //     }
        //   }
        // }
        // points = Object.assign(points, global_mirrored_points)
        //
        // removing temporary points
        // const filtered = {}
        // for (const [k, p] of Object.entries(points)) {
        //   if (p.meta.skip) continue
        //   filtered[k] = p
        // }
        //
        // apply autobind
        // perform_autobind(filtered, units)

        Ok(points)
    }

    pub fn parse_axis(
        &self,
        mirror: Option<Mirror>,
        name: String,
        points: &IndexMap<String, Point>,
        units: &IndexMap<String, f64>,
    ) -> Result<Point> {
        // let Some(mirror) = mirror else {
        //     return Ok(mirror.into());
        // };

        todo!()
    }

    pub fn units(&self) -> Units {
        self.units.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

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
                println!("skipping {:?}", points_file);
                continue;
            }

            println!("Reading {:?}", points_file);
            let expected = fs::read_to_string(points_file).unwrap();
            let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();

            let actual = config.parse_points().unwrap();

            assert_json_eq!(expected, json!(actual));
            // println!("expected: {:#?}", expected);
            // println!("actual: {:#?}", actual);
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

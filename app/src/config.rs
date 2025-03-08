use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{preprocess::preprocess, yaml::preprocess_extends};
use crate::{
    anchor::parse_anchored,
    point::Point,
    types::{Meta, Mirror, Points, Unit, Units},
    units::evaluate_mathnum,
    zone::perform_mirror,
    Error, Result,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Units>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Units>,
    pub points: Points,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<Unit>,
}

impl Config {
    pub fn process(config: impl ToString) -> Result<IndexMap<String, Point>> {
        println!("Preprocessing config...");
        let config = preprocess_extends(config.to_string())?;
        println!("Parsing config...");
        let config = Config::parse(config)?;
        println!("Parsing points...");
        config.parse_points()
    }

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

        // rendering zones
        for (name, zone) in self.points.zones.iter() {
            println!("Processing zone {name}...");
            let mut zone = zone.clone();
            zone.name = Some(name.to_string());

            // extracting keys that are handled here, not at the zone render level
            let anchor = match zone.anchor {
                Some(ref anchor) => {
                    let name = format!("points.zones.{name}.anchor");
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
                let name = if name.ends_with("_default") {
                    // remove everything after the first "_default"
                    name.split_once("_default").unwrap().0
                } else {
                    name.as_str()
                };
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
            points.extend(renamed_points.clone());

            // per-zone mirroring for the new keys
            if let Some(axis) = self.parse_axis(
                mirror,
                format!("points.zones.{name}.mirror"),
                &points,
                &units,
            )? {
                let mut mirrored_points = IndexMap::new();
                for (_new_name, new_point) in renamed_points.iter() {
                    let (mname, mp) = perform_mirror(new_point, axis);
                    if let Some(mp) = mp {
                        mirrored_points.insert(mname, mp);
                    }
                }

                points.extend(mirrored_points);
            }
        }

        // applying global rotation
        if let Some(global_rotate) = global_rotate {
            points = points
                .iter()
                .map(|(name, p)| (name.clone(), p.rotated(global_rotate, None, false)))
                .collect();
        }

        // global mirroring for points that haven't been mirrored yet
        let global_mirror = self.points.mirror.clone();
        let global_axis =
            self.parse_axis(global_mirror, "points.mirror".to_string(), &points, &units)?;
        let global_mirrored_points: IndexMap<_, _> = points
            .iter()
            .filter_map(|(_, point)| {
                if let Some(meta) = &point.meta {
                    if meta.mirrored.unwrap_or_default() {
                        let (mirrored_name, mirrored_point) = perform_mirror(point, global_axis?);
                        mirrored_point.map(|mirrored_point| (mirrored_name, mirrored_point))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        points.extend(global_mirrored_points);

        // removing temporary points
        points = points
            .into_iter()
            .filter(|(_, point)| !point.is_skip())
            .collect();

        // apply autobind
        perform_autobind(&mut points, units)?;

        Ok(points)
    }

    pub fn parse_axis(
        &self,
        mirror: Option<Mirror>,
        name: String,
        points: &IndexMap<String, Point>,
        units: &IndexMap<String, f64>,
    ) -> Result<Option<f64>> {
        let Some(mirror) = mirror else {
            return Ok(None);
        };

        let axis = parse_anchored(&mirror, name.clone(), points, None, false, units)?;
        let mut axis_x = axis.x.unwrap_or_default();

        if let Some(distance) = mirror.distance {
            let distance = distance.eval_as_number(&format!("{name}.distance"), units)?;
            axis_x += distance / 2.0;
        }

        Ok(Some(axis_x))
    }

    pub fn units(&self) -> Units {
        self.units.clone().unwrap_or_default()
    }
}

fn mirrorzone(p: &Point) -> String {
    let mirrored = match &p.meta {
        Some(meta) => meta.mirrored.unwrap_or_default(),
        None => false,
    };
    let zone_name = match &p.meta {
        Some(meta) => meta.zone_name(),
        None => "".to_string(),
    };

    let prefix = if mirrored { "mirror_" } else { "" };
    format!("{}{}", prefix, zone_name)
}

pub fn perform_autobind(
    points: &mut IndexMap<String, Point>,
    units: IndexMap<String, f64>,
) -> Result<()> {
    let mut bounds = IndexMap::new();
    let mut col_lists = IndexMap::new();

    // round one: get column upper/lower bounds and per-zone column lists
    perform_autobind_round1(points, &mut bounds, &mut col_lists)?;
    // round two: apply autobind as appropriate
    perform_autobind_round2(points, &bounds, &col_lists, &units)?;

    Ok(())
}

fn perform_autobind_round1(
    points: &mut IndexMap<String, Point>,
    bounds: &mut IndexMap<String, IndexMap<String, (f64, f64)>>,
    col_lists: &mut IndexMap<String, Vec<String>>,
) -> Result<()> {
    for (_, point) in points.iter_mut() {
        let zone = mirrorzone(point);
        let col = point.meta_col_name();

        if !bounds.contains_key(&zone) {
            bounds.insert(zone.clone(), IndexMap::new());
        }
        if !bounds[&zone].contains_key(&col) {
            bounds
                .get_mut(&zone)
                .unwrap()
                .insert(col.to_string(), (f64::INFINITY, f64::NEG_INFINITY));
        }
        if !col_lists.contains_key(&zone) {
            let value: Vec<_> = point.meta_zone_columns().keys().cloned().collect();
            col_lists.insert(zone.clone(), value);
        }

        let (min, max) = bounds.get_mut(&zone).unwrap().get_mut(&col).unwrap();
        *min = min.min(point.y.unwrap_or_default());
        *max = max.max(point.y.unwrap_or_default());
    }

    Ok(())
}

fn perform_autobind_round2(
    points: &mut IndexMap<String, Point>,
    bounds: &IndexMap<String, IndexMap<String, (f64, f64)>>,
    col_lists: &IndexMap<String, Vec<String>>,
    units: &IndexMap<String, f64>,
) -> Result<()> {
    for (_, point) in points.iter_mut() {
        let autobind = point.meta.as_ref().map(|meta| meta.autobind);
        if let Some(autobind) = autobind {
            let zone = mirrorzone(point);
            let col = point.meta_col_name();
            let col_list = col_lists[zone.as_str()].clone();
            let col_bounds = bounds[zone.as_str()][&col];

            let Some(mut bind) = point.meta_bind(units)? else {
                continue;
            };

            // up
            if bind[0] == -1.0 {
                if point.y.unwrap_or_default() < col_bounds.1 {
                    bind[0] = autobind.unwrap_or_default();
                } else {
                    bind[0] = 0.0;
                }
            }

            // down
            if bind[2] == -1.0 {
                if point.y.unwrap_or_default() > col_bounds.0 {
                    bind[2] = autobind.unwrap_or_default();
                } else {
                    bind[2] = 0.0;
                }
            }

            // left
            if bind[3] == -1.0 {
                bind[3] = 0.0;
                let col_index = col_list.iter().position(|c| *c == col).unwrap_or_default();
                if col_index > 0 {
                    if let Some(left) = bounds.get(&zone) {
                        let col_name = col_list[col_index - 1].clone();
                        if let Some(left) = left.get(col_name.as_str()) {
                            if left.0 <= point.y.unwrap_or_default()
                                && point.y.unwrap_or_default() <= left.1
                            {
                                bind[3] = autobind.unwrap_or_default();
                            }
                        }
                    }
                }
            }

            // right
            if bind[1] == -1.0 {
                bind[1] = 0.0;
                let col_index = col_list.iter().position(|c| *c == col).unwrap_or_default();
                if col_index < col_list.len() - 1 {
                    if let Some(right) = bounds.get(&zone) {
                        let col_name = col_list[col_index + 1].clone();
                        if let Some(right) = right.get(col_name.as_str()) {
                            if right.0 <= point.y.unwrap_or_default()
                                && point.y.unwrap_or_default() <= right.1
                            {
                                bind[1] = autobind.unwrap_or_default();
                            }
                        }
                    }
                }
            }

            let meta = point.meta.as_mut().unwrap();
            meta.bind = Some(bind.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_sweeplike() {
        let config = include_str!("../fixtures/sweep-like.yaml");
        let config = Config::process(config).unwrap();
        let ours = serde_json::to_value(config).unwrap();
        fs::write(
            "fixtures/sweep-like___output.json",
            serde_json::to_string_pretty(&ours).unwrap(),
        )
        .unwrap();
        let theirs = include_str!("../fixtures/sweep-like___points.json");
        let theirs: serde_json::Value = serde_json::from_str(theirs).unwrap();

        assert_json_eq!(theirs, ours);
    }

    #[test]
    fn test_parse_empty() {
        let config = include_str!("../fixtures/empty.yaml");
        let config = Config::process(config).unwrap();
        let ours = serde_json::to_value(config).unwrap();
        fs::write(
            "fixtures/empty___output.json",
            serde_json::to_string_pretty(&ours).unwrap(),
        )
        .unwrap();
        let theirs = include_str!("../fixtures/empty___points.json");
        let theirs: serde_json::Value = serde_json::from_str(theirs).unwrap();

        assert_json_eq!(theirs, ours);
    }

    #[test]
    fn test_parse_absolem() {
        let config = include_str!("../fixtures/absolem-mini.yaml");
        let config = Config::process(config).unwrap();
        // println!("{}", serde_json::to_string_pretty(&config).unwrap());

        let point = config.get("matrix_inner_bottom").unwrap();
        let r = point.r.unwrap();
        assert_eq!(r, -56.0);
        println!("{:#?}", point.y.unwrap());

        // let ours = serde_json::to_value(config).unwrap();
        // fs::write(
        //     "fixtures/absolem___output.json",
        //     serde_json::to_string_pretty(&ours).unwrap(),
        // )
        // .unwrap();
        // let theirs = include_str!("../fixtures/absolem___points.json");
        // let theirs: serde_json::Value = serde_json::from_str(theirs).unwrap();
        //
        // assert_json_eq!(theirs, ours);
    }

    #[test]
    fn test_parse_zeph() {
        let config = include_str!("../fixtures/zeph.yaml");
        let config = Config::process(config).unwrap();
        println!("{}", serde_json::to_string_pretty(&config).unwrap());

        // let ours = serde_json::to_value(config).unwrap();
        // fs::write(
        //     "fixtures/zeph___output.json",
        //     serde_json::to_string_pretty(&ours).unwrap(),
        // )
        // .unwrap();
        // let theirs = include_str!("../fixtures/zeph___points.json");
        // let theirs: serde_json::Value = serde_json::from_str(theirs).unwrap();
        //
        // assert_json_eq!(theirs, ours);
    }

    #[test]
    fn test_parse() {
        for file in fs::read_dir("fixtures/points").unwrap() {
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

            let contents = fs::read_to_string(file.path()).unwrap();
            let config = Config::parse(contents).unwrap();

            let file = file.path();
            let parent = file.parent().unwrap_or(Path::new(""));
            let file_stem = file.file_stem().unwrap().to_str().unwrap();
            let points_file = parent.join(format!("{file_stem}___points.json"));

            if !points_file.exists() {
                continue;
            }

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
        let config = include_str!("../fixtures/points/basic_2x2.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn parse_with_adjustments() {
        let config = include_str!("../fixtures/points/adjustments.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn parse_autobind() {
        let config = include_str!("../fixtures/points/autobind.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_default() {
        let config = include_str!("../fixtures/points/default.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_mirrors() {
        let config = include_str!("../fixtures/points/mirrors.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_rotations() {
        let config = include_str!("../fixtures/points/rotations.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_samename() {
        let config = include_str!("../fixtures/points/samename.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_parse_units() {
        let config = include_str!("../fixtures/points/units.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    #[ignore = "$unset is not implemented"]
    fn test_parse_unset() {
        let config = include_str!("../fixtures/points/overrides.yaml");
        let config = Config::parse(config).unwrap();
        println!("{:#?}", config);
    }
}

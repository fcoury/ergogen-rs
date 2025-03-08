use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    aggregator::average,
    point::Point,
    types::{Asym, Unit},
};
use crate::{Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Anchor {
    Ref(String),
    Single(Box<AnchorItem>),
    Multiple(Vec<AnchorItem>),
}

pub trait Anchored {
    fn ref_(&self) -> Option<Anchor>;
    fn aggregate(&self) -> Option<Aggregate>;
    fn orient(&self) -> Option<Unit>;
    fn shift(&self) -> Option<Shift>;
    fn rotate(&self) -> Option<Unit>;
    fn affect(&self) -> Option<Vec<AffectType>>;
    fn resist(&self) -> Option<bool>;
    fn asym(&self) -> Option<Asym>;
}

pub trait AnchoredDebug: Anchored + std::fmt::Debug {}

impl<T: Anchored + std::fmt::Debug> AnchoredDebug for T {}

pub fn parse_anchored(
    anchor: &dyn AnchoredDebug,
    name: String,
    points: &IndexMap<String, Point>,
    start: Option<Point>,
    mirror: bool,
    units: &IndexMap<String, f64>,
) -> Result<Point> {
    let mut point = start.clone().unwrap_or_default();

    if anchor.ref_().is_some() && anchor.aggregate().is_some() {
        return Err(Error::AnchorParse {
            name: name.clone(),
            message: format!(
                r#"Fields "ref" and "aggregate" cannot appear together in anchor "{name}"!"#
            ),
        });
    }

    match anchor.ref_() {
        Some(Anchor::Ref(ref_)) => {
            let parsed_ref = handle_mirror_ref(&ref_, mirror);
            let Some(ref_point) = points.get(&parsed_ref) else {
                let known_points = points
                    .keys()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(Error::AnchorParse {
                    name: name.clone(),
                    message: format!(
                        r#"Unknown point reference "{parsed_ref}". Known points: {known_points}"#
                    ),
                });
            };
            point = ref_point.clone();
        }
        Some(anchor) => {
            point = anchor.parse(name.clone(), points, Some(point), mirror, units)?;
        }
        None => {}
    }

    if let Some(agg) = anchor.aggregate() {
        let mut parts = vec![];

        for (index, part) in agg.parts.iter().enumerate() {
            let part = Anchor::Single(Box::new(part.clone()));
            parts.push(part.parse(
                format!("{}[{}]", name, index),
                points,
                Some(point.clone()),
                mirror,
                units,
            )?);
        }

        let method = agg.method.clone().unwrap_or_default();
        point = method.apply(&agg, format!("{name}.aggregate"), &parts)?;
    };

    let resist = anchor.resist().unwrap_or_default();

    // TODO: refactor
    #[allow(clippy::too_many_arguments)]
    fn rotator(
        config: &Unit,
        name: String,
        point: Point,
        start: Option<Point>,
        points: &IndexMap<String, Point>,
        resist: bool,
        mirror: bool,
        units: &IndexMap<String, f64>,
    ) -> Result<Point> {
        match config.eval(units) {
            // simple case: number gets added to point rotation
            crate::types::EvalResult::Number(angle) => {
                let mut point = point.clone();
                point.rotate(angle, None, resist);
                Ok(point)
            }
            // recursive case: points turns "towards" target anchor
            crate::types::EvalResult::Ref(ref_) => {
                let anchor = Anchor::Ref(ref_);
                let target = anchor.parse(name.clone(), points, start, mirror, units)?;
                let mut point = point.clone();
                point.r = Some(point.angle(&target));
                Ok(point)
            }
        }
    }

    if let Some(orient) = anchor.orient() {
        point = rotator(
            &orient,
            format!("{name}.orient"),
            point,
            start.clone(),
            points,
            resist,
            mirror,
            units,
        )?;
    }

    if let Some(shift) = anchor.shift() {
        match shift {
            Shift::XY(x, y) => {
                let x = x
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.clone(),
                        message: format!(r#"Invalid shift value for x: "{x}""#),
                    })?;
                let y = y
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.clone(),
                        message: format!(r#"Invalid shift value for y: "{y}""#),
                    })?;
                point.shift((x, y), Some(resist), None);
            }
            Shift::Number(n) => {
                let n = n
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.clone(),
                        message: format!(r#"Invalid shift value: "{n}""#),
                    })?;
                point.shift((n, n), Some(resist), None);
            }
        }
    }

    if let Some(rotate) = anchor.rotate() {
        point = rotator(
            &rotate,
            format!("{name}.rotate"),
            point,
            start.clone(),
            points,
            resist,
            mirror,
            units,
        )?;
    }

    if let Some(affect) = anchor.affect() {
        let candidate = point.clone();
        point = start.unwrap_or_default().clone();
        point.meta = candidate.meta;

        for field in affect {
            match field {
                AffectType::X => point.y = candidate.x,
                AffectType::Y => point.y = candidate.y,
                AffectType::R => point.r = candidate.r,
            }
        }
    }

    Ok(point)
}

impl Anchor {
    pub fn parse(
        &self,
        name: String,
        points: &IndexMap<String, Point>,
        start: Option<Point>,
        mirror: bool,
        units: &IndexMap<String, f64>,
    ) -> Result<Point> {
        //     a.unexpected(raw, name, ['ref', 'aggregate', 'orient', 'shift', 'rotate', 'affect', 'resist'])
        let anchor = match self {
            Self::Ref(_) => AnchorItem {
                ref_: Some(self.clone()),
                ..Default::default()
            },
            Self::Multiple(items) => {
                let mut current = start.clone().unwrap_or_default();
                let mut index = 1;
                for step in items {
                    let anchor = Anchor::Single(Box::new(step.clone()));
                    current = anchor.parse(
                        format!("{}[{}]", name, index),
                        points,
                        Some(current),
                        mirror,
                        units,
                    )?;
                    index += 1;
                }
                return Ok(current);
            }
            Self::Single(item) => (**item).clone(),
        };

        parse_anchored(&anchor, name, points, start, mirror, units)
    }
}

fn handle_mirror_ref(ref_: &str, mirror: bool) -> String {
    if mirror {
        ref_.strip_prefix("mirror_").unwrap_or(ref_).to_string()
    } else {
        ref_.to_string()
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::Single(Box::new(AnchorItem {
            ref_: None,
            aggregate: None,
            orient: None,
            shift: None,
            rotate: None,
            affect: None,
            resist: None,
        }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AnchorItem {
    /// Starting point from where the anchor will perform its additional modifications. This field
    /// is parsed as an anchor itself, recursively. So in its easiest form, it can be a string to
    /// designate an existing starting point by name (more on names later), but it can also be a
    /// full nested anchor if so desired.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ref")]
    ref_: Option<Anchor>,

    /// Alternative to ref when the combination of several locations is required as the starting
    /// point for further adjustment. They're mutually exclusive, so we can use either ref or
    /// aggregate in any given anchor. The aggregate field is always an object, containing:
    ///
    /// - a parts array containing the sub-anchors we want to aggregate, and
    /// - a method string to indicate how we want to aggregate them.
    ///
    /// The only method implemented so far is average, which is the default anyway, so the method
    /// can be omitted for now.
    ///
    /// Note: Averaging applies to both the x/y coordinates and the r rotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregate: Option<Aggregate>,

    /// Kind of pre-rotation, meaning it happens before any shifting is done. The value can be:
    ///
    /// - a number, in which case that number is simply added to the current rotation of the
    ///   in-progress point calculation; or
    /// - a sub-anchor, in which case the point "turns towards" the point we reference (meaning its
    ///   rotations will be exactly set to hit that point if a line was projected from it).
    #[serde(skip_serializing_if = "Option::is_none")]
    orient: Option<Unit>,

    /// Shifting (or, more formally, translating) the point on the XY plane. The value can be:
    ///
    /// - a array of exactly two numbers, specifying the x and y shift, respectively, or
    /// - a single number, which would get parsed as [number, number].
    ///
    /// Caution: It's important that shifting happens according to the current rotation of the
    /// point. By default, a 0° rotation is "looking up", so that positive x shifts move it to the
    /// right, negative x shifts to the left, positive y shifts up, negative y shifts down. But if
    /// r=90° (so the point is "looking left", as, remember, rotation works counter-clockwise),
    /// then a positive x shift would move it upward.
    #[serde(skip_serializing_if = "Option::is_none")]
    shift: Option<Shift>,

    /// Kind of post-rotation after shifting, as opposed to how orient was the pre-rotation.
    /// Otherwise, it works the exact same way.
    #[serde(skip_serializing_if = "Option::is_none")]
    rotate: Option<Unit>,

    /// Specify an override to what fields we want to affect during the current anchor calculation.
    /// The value can be:
    ///
    /// - a string containing a subset of the characters x, y, or r only; or
    /// - an array containing a subset of the one letter strings "x", "y", or "r" only.
    ///
    /// Tip: Let's say you have a point rotated 45° and want to shift is "visually right". You
    /// could either reset its rotation via orient, then shift, then reset the rotation with
    /// rotate; or, you could do the shift and then declare that this whole anchor only affects
    /// "x". The amount of shifting wouldn't be the same, but the important thing is that you could
    /// constrain the movement to the X axis this way.
    ///
    /// Or let's say you want to copy the rotation of another, already existing point into your
    /// current anchor calculation. You can do so using a multi-anchor (see above), referencing the
    /// existing point in the second part, and then declare affect: "r" to prevent it from
    /// overwriting anything else, thereby setting just the rotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    affect: Option<Vec<AffectType>>,

    /// States that we do not want the special treatment usually afforded to mirrored points. We'll
    /// get to mirroring in a second, but from an anchor perspective, all we need to know is that
    /// shifting and orienting/rotating are all mirrored for mirrored points, to keep things
    /// symmetric. So if we specify a shift of [1, 1] on a mirrored point, what actually gets
    /// applied is [-1, 1], and rotations are clockwise (read, counter-counter-clockwise) in those
    /// cases, too. But if we don't want this behavior, (say, because PCB footprints go on the
    /// same, upward facing side of the board, no matter the half) we can resist the special
    /// treatment
    #[serde(skip_serializing_if = "Option::is_none")]
    resist: Option<bool>,
}

impl Anchored for AnchorItem {
    fn ref_(&self) -> Option<Anchor> {
        self.ref_.clone()
    }

    fn aggregate(&self) -> Option<Aggregate> {
        self.aggregate.clone()
    }

    fn orient(&self) -> Option<Unit> {
        self.orient.clone()
    }

    fn shift(&self) -> Option<Shift> {
        self.shift.clone()
    }

    fn rotate(&self) -> Option<Unit> {
        self.rotate.clone()
    }

    fn affect(&self) -> Option<Vec<AffectType>> {
        self.affect.clone()
    }

    fn resist(&self) -> Option<bool> {
        self.resist
    }

    fn asym(&self) -> Option<Asym> {
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum AggregateMethod {
    #[default]
    #[serde(rename = "average")]
    Average,
    #[serde(rename = "intersect")]
    Intersect,
}

impl AggregateMethod {
    // TODO: may need to extract a trait with parts and method to make methods generic
    fn apply(&self, _agg: &Aggregate, name: String, parts: &[Point]) -> Result<Point> {
        match self {
            Self::Average => Ok(average(parts)),
            Self::Intersect => {
                // a line is generated from a point by taking their
                // (rotated) Y axis. The line is not extended to
                // +/- Infinity as that doesn't work with makerjs.
                // An arbitrary offset of 1 meter is considered
                // sufficient for practical purposes, and the point
                // coordinates are used as pivot point for the rotation.
                fn get_line_from_point(
                    point: &Point,
                    offset: Option<f64>,
                ) -> maker_rs::paths::Line {
                    let offset = offset.unwrap_or(1000.0);
                    let x = point.x.unwrap_or_default();
                    let y = point.y.unwrap_or_default();
                    let origin = (x, y);
                    let p1 = (x, y - offset);
                    let p2 = (x, y + offset);

                    let line = maker_rs::paths::Line {
                        origin: p1,
                        end: p2,
                    };

                    maker_rs::path::rotate(&line, point.r.unwrap_or_default(), Some(origin))
                }

                let line1 = get_line_from_point(&parts[0], None);
                let line2 = get_line_from_point(&parts[1], None);
                let intersection_points = maker_rs::path::intersection(&line1, &line2, None);

                if intersection_points.is_empty() {
                    return Err(Error::AnchorParse {
                        name: name.clone(),
                        message: format!("The points under \"{name}.parts\" do not intersect!"),
                    });
                }

                let intersection_point = intersection_points.first().unwrap();

                Ok(Point {
                    x: Some(intersection_point.0),
                    y: Some(intersection_point.1),
                    ..Default::default()
                })
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aggregate {
    parts: Vec<AnchorItem>,
    method: Option<AggregateMethod>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Shift {
    XY(Unit, Unit),
    Number(Unit),
}

impl Shift {
    pub fn eval_as_numbers(&self, name: &str, units: &IndexMap<String, f64>) -> Result<(f64, f64)> {
        match self {
            Shift::XY(x, y) => {
                let x = x
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.to_owned(),
                        message: format!(r#"Invalid shift value for x: "{x}""#),
                    })?;
                let y = y
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.to_owned(),
                        message: format!(r#"Invalid shift value for y: "{y}""#),
                    })?;
                Ok((x, y))
            }
            Shift::Number(n) => {
                let n = n
                    .eval(units)
                    .as_number()
                    .ok_or_else(|| Error::AnchorParse {
                        name: name.to_owned(),
                        message: format!(r#"Invalid shift value: "{n}""#),
                    })?;
                Ok((n, n))
            }
        }
    }
}

impl Default for Shift {
    fn default() -> Self {
        Self::Number(Unit::Number(0.0))
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum AffectType {
    #[serde(rename = "x")]
    X,
    #[serde(rename = "y")]
    Y,
    #[serde(rename = "r")]
    R,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_anchor() {
        let anchor = Anchor::Multiple(vec![AnchorItem {
            ref_: Some(Anchor::Ref("thumb_far_home".to_string())),
            shift: Some(Shift::XY(
                Unit::Expression("-ks * 0.725 - 0.028".to_string()),
                Unit::Expression("-kp * 0.48 + 0.023".to_string()),
            )),
            affect: Some(vec![AffectType::X, AffectType::Y]),
            ..Default::default()
        }]);

        let serialized = serde_json::to_string_pretty(&anchor).unwrap();
        println!("{}", serialized);
        let deserialized: Anchor = serde_json::from_str(&serialized).unwrap();
        println!("{:#?}", deserialized);
    }

    #[test]
    fn deserialize_anchor() {
        let anchor = r#"
        - aggregate:
            method: intersect
            parts:
              - ref: mcu_cover_top_left
              - ref: mcu_cover_bottom_right
        "#;

        let deserialized: Anchor = serde_yaml::from_str(anchor).unwrap();
        println!("{:#?}", deserialized);
    }
}

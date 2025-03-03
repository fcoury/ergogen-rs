use serde::{Deserialize, Serialize};

use super::{Unit, Units};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Anchor {
    Single(Box<AnchorItem>),
    Multiple(Vec<AnchorItem>),
}

impl Anchor {
    pub fn parse(&self, units: &Units) -> Anchor {
        todo!()
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::Single(Box::new(AnchorItem {
            r#ref: None,
            aggregate: None,
            orient: None,
            shift: None,
            rotate: None,
            affect: None,
            resist: None,
        }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnchorItem {
    /// starting point from where the anchor will perform its additional modifications. This field
    /// is parsed as an anchor itself, recursively. So in its easiest form, it can be a string to
    /// designate an existing starting point by name (more on names later), but it can also be a
    /// full nested anchor if so desired.
    r#ref: Option<String>,

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
    aggregate: Option<Aggregate>,

    /// Kind of pre-rotation, meaning it happens before any shifting is done. The value can be:
    ///
    /// - a number, in which case that number is simply added to the current rotation of the
    ///   in-progress point calculation; or
    /// - a sub-anchor, in which case the point "turns towards" the point we reference (meaning its
    ///   rotations will be exactly set to hit that point if a line was projected from it).
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
    shift: Option<Shift>,

    /// Kind of post-rotation after shifting, as opposed to how orient was the pre-rotation.
    /// Otherwise, it works the exact same way.
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
    affect: Option<Vec<AffectType>>,

    /// States that we do not want the special treatment usually afforded to mirrored points. We'll
    /// get to mirroring in a second, but from an anchor perspective, all we need to know is that
    /// shifting and orienting/rotating are all mirrored for mirrored points, to keep things
    /// symmetric. So if we specify a shift of [1, 1] on a mirrored point, what actually gets
    /// applied is [-1, 1], and rotations are clockwise (read, counter-counter-clockwise) in those
    /// cases, too. But if we don't want this behavior, (say, because PCB footprints go on the
    /// same, upward facing side of the board, no matter the half) we can resist the special
    /// treatment
    resist: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AggregateMethod {
    Average,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AffectType {
    X,
    Y,
    R,
}

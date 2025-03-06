use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    anchor::{AffectType, Aggregate, Anchor, Anchored, Shift},
    Asym, Key, Unit,
};
use crate::Result;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Point {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<AnchorInfo>,
}

impl Point {
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
}

impl From<Point> for (f64, f64) {
    fn from(point: Point) -> Self {
        (point.x.unwrap_or_default(), point.y.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnchorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_: Option<Anchor>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asym: Option<Asym>,
    pub colrow: String,
    pub name: String,
    pub mirrored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affect: Option<Vec<AffectType>>,
    // zone: ParsedZone,
}

impl AnchorInfo {
    fn from(key: Key, units: &IndexMap<String, f64>) -> Result<Self> {
        let mut meta = AnchorInfo::default();
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

        // TODO: How to handle mirroring here?
        // if let Some(mirrored) = key.mirror {
        //     meta.mirrored = mirrored;
        // }

        Ok(meta)
    }
}

impl Anchored for AnchorInfo {
    fn ref_(&self) -> Option<Anchor> {
        self.ref_.clone()
    }

    fn aggregate(&self) -> Option<Aggregate> {
        None
    }

    fn orient(&self) -> Option<Unit> {
        Some(Unit::Number(self.orient))
    }

    fn shift(&self) -> Option<Shift> {
        let x = Unit::Number(self.shift.0);
        let y = Unit::Number(self.shift.1);
        Some(Shift::XY(x, y))
    }

    fn rotate(&self) -> Option<Unit> {
        Some(Unit::Number(self.rotate))
    }

    fn affect(&self) -> Option<Vec<AffectType>> {
        self.affect.clone()
    }

    fn resist(&self) -> Option<bool> {
        // TODO: do we need this?
        None
    }

    fn asym(&self) -> Option<Asym> {
        self.asym
    }
}

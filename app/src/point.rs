use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    anchor::{AffectType, Aggregate, Anchor, Anchored, Shift},
    types::{Asym, Key, Unit},
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
        let mirrored = self
            .meta
            .as_ref()
            .is_some_and(|meta| meta.mirrored.unwrap_or_default());
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

        let x = if !resist
            && self
                .meta
                .as_ref()
                .is_some_and(|meta| meta.mirrored.unwrap_or_default())
        {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stagger: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orient: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<f64>,
    // TODO: adjust: {}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autobind: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asym: Option<Asym>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colrow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affect: Option<Vec<AffectType>>,
    // zone: ParsedZone,
}

impl AnchorInfo {
    #[allow(unused)]
    fn from(key: Key, units: &IndexMap<String, f64>) -> Result<Self> {
        let mut meta = AnchorInfo::default();
        let key_name = key.name.unwrap_or("key".to_string());

        if let Some(stagger) = key.stagger {
            meta.stagger = Some(stagger.eval_as_number(&format!("{key_name}.stagger"), units)?);
        }

        if let Some(spread) = key.spread {
            meta.spread = Some(spread.eval_as_number(&format!("{key_name}.spread"), units)?);
        }

        if let Some(origin) = key.origin {
            let (x, y) = origin;
            let x = x.eval_as_number("key.origin.x", units)?;
            let y = y.eval_as_number("key.origin.y", units)?;
            Some(meta.origin = Some((x, y)));
        }

        if let Some(orient) = key.orient {
            meta.orient = Some(orient.eval_as_number(&format!("{key_name}.orient"), units)?);
        }

        if let Some(shift) = key.shift {
            meta.shift = Some(shift.eval_as_numbers(&format!("{key_name}.shift"), units)?);
        }

        if let Some(rotate) = key.rotate {
            meta.rotate = Some(rotate.eval_as_number(&format!("{key_name}.rotate"), units)?);
        }

        if let Some(width) = key.width {
            meta.width = Some(width.eval_as_number(&format!("{key_name}.rotate"), units)?);
        }

        if let Some(height) = key.height {
            meta.height = Some(height.eval_as_number(&format!("{key_name}.height"), units)?);
        }

        if let Some(padding) = key.padding {
            meta.padding = Some(padding.eval_as_number(&format!("{key_name}.padding"), units)?);
        }

        if let Some(autobind) = key.autobind {
            meta.autobind = Some(autobind.eval_as_number(&format!("{key_name}.autobind"), units)?);
        }

        if let Some(skip) = key.skip {
            meta.skip = Some(skip);
        }

        meta.asym = key.asym;

        if let Some(colrow) = key.colrow {
            meta.colrow = Some(colrow);
        }

        meta.name = Some(key_name);

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
        self.orient.map(Unit::Number)
    }

    fn shift(&self) -> Option<Shift> {
        let shift = self.shift?;
        let x = Unit::Number(shift.0);
        let y = Unit::Number(shift.1);
        Some(Shift::XY(x, y))
    }

    fn rotate(&self) -> Option<Unit> {
        self.rotate.map(Unit::Number)
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

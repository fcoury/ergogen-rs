use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    anchor::{AffectType, Aggregate, Anchor, Anchored, Shift},
    types::{Asym, Unit},
};
use crate::{
    types::Bind,
    zone::{Column, ParsedKey, Zone},
    Result,
};

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
    pub fn is_skip(&self) -> bool {
        self.meta
            .as_ref()
            .is_some_and(|meta| meta.skip.unwrap_or_default())
    }

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
        let (mut x, mut y) = pos;
        let relative = relative.unwrap_or(true);
        let resist = resist.unwrap_or(false);

        x = if !resist
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
            (x, y) = maker_rs::point::rotate((x, y), self.r.unwrap_or_default(), None);
        }

        self.x = Some(self.x.unwrap_or_default() + x);
        self.y = Some(self.y.unwrap_or_default() + y);
        self
    }

    pub fn mirror(&mut self, x_axis: f64) -> &mut Self {
        let x = self.x.unwrap_or_default();
        self.x = Some(2.0 * x_axis - x);
        self
    }

    pub fn mirrored(&self, x_axis: f64) -> Self {
        let x = self.x.unwrap_or_default();
        let x = 2.0 * x_axis - x;

        Point {
            x: Some(x),
            y: self.y,
            r: self.r,
            meta: self.meta.clone(),
        }
    }

    pub fn meta_bind(&self, units: &IndexMap<String, f64>) -> Result<Option<[f64; 4]>> {
        let option_result = self
            .meta
            .as_ref()
            .and_then(|meta| meta.bind.as_ref())
            .map(|bind| bind.resolve(units));

        match option_result {
            Some(result) => result.map(Some),
            None => Ok(None),
        }
    }

    pub fn meta_col_name(&self) -> String {
        self.meta.as_ref().map_or("".to_string(), |meta| {
            meta.colrow.clone().unwrap_or_default()
        })
    }

    pub fn meta_zone_columns(&self) -> IndexMap<String, Column> {
        self.meta.as_ref().map_or(IndexMap::new(), |meta| {
            meta.zone
                .as_ref()
                .map_or(IndexMap::new(), |zone| zone.columns())
        })
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjust: Option<Box<AnchorInfo>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<Box<Zone>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<Bind>,
}

impl From<ParsedKey> for AnchorInfo {
    fn from(key: ParsedKey) -> Self {
        AnchorInfo::from_parsed_key(key)
    }
}

impl AnchorInfo {
    pub fn zone_name(&self) -> String {
        self.zone
            .as_ref()
            .map_or("".to_string(), |zone| zone.name.clone().unwrap_or_default())
    }

    pub fn from_parsed_key(key: ParsedKey) -> Self {
        AnchorInfo {
            stagger: key.stagger,
            spread: key.spread,
            origin: key.origin,
            orient: key.orient,
            shift: key.shift,
            rotate: key.rotate,
            width: key.width,
            height: key.height,
            padding: key.padding,
            autobind: key.autobind,
            skip: key.skip,
            asym: key.asym,
            colrow: key.colrow,
            name: key.name,
            zone: key.zone.map(Box::new),
            // TODO: How to handle mirroring here?
            // if let Some(mirrored) = key.mirror {
            //     meta.mirrored = mirrored;
            // }
            ..Default::default()
        }
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

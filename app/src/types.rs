use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::anchor::{AffectType, Aggregate, Anchor, Anchored, Shift};
use crate::point::AnchorInfo;
use crate::template::process_templates;
use crate::zone::{Column, ParsedKey, Zone};

use crate::{expr::evaluate_expression, Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<StringOrFloat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    footprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    switch: Option<serde_yaml::Value>,
}

pub type Units = IndexMap<String, Unit>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrFloat {
    String(String),
    Float(f64),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Unit {
    Number(f64),
    Expression(String),
}

impl Default for Unit {
    fn default() -> Self {
        Unit::Number(0.0)
    }
}

impl Unit {
    pub fn eval(&self, units: &IndexMap<String, f64>) -> EvalResult {
        match self {
            Unit::Number(num) => EvalResult::Number(*num),
            Unit::Expression(expr) => match evaluate_expression(expr, units) {
                Ok(num) => EvalResult::Number(num),
                Err(_) => EvalResult::Ref(expr.clone()),
            },
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Unit::Number(_))
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Unit::Number(num) => Some(*num),
            _ => None,
        }
    }

    pub fn eval_as_number(&self, name: &str, units: &IndexMap<String, f64>) -> Result<f64> {
        match self.eval(units) {
            EvalResult::Number(num) => Ok(num),
            EvalResult::Ref(expr) => Err(Error::UnitParse(name.to_owned(), expr)),
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Number(num) => write!(f, "{}", num),
            Unit::Expression(expr) => write!(f, "{}", expr),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Bind {
    Number(Unit),
    HorizontalVertical(Unit, Unit),
    TopRightBottomLeft(Unit, Unit, Unit, Unit),
}

impl Bind {
    pub fn resolve(&self, units: &IndexMap<String, f64>) -> Result<[f64; 4]> {
        match self {
            Bind::Number(num) => {
                let num = num.eval(units).resolve_as_number("bind")?;
                Ok([num, num, num, num])
            }
            Bind::HorizontalVertical(hor, ver) => {
                let hor = hor.eval(units).resolve_as_number("bind")?;
                let ver = ver.eval(units).resolve_as_number("bind")?;
                Ok([ver, hor, ver, hor])
            }
            Bind::TopRightBottomLeft(top, right, bottom, left) => {
                let top = top.eval(units).resolve_as_number("bind")?;
                let right = right.eval(units).resolve_as_number("bind")?;
                let bottom = bottom.eval(units).resolve_as_number("bind")?;
                let left = left.eval(units).resolve_as_number("bind")?;
                Ok([top, right, bottom, left])
            }
        }
    }
}

impl From<[f64; 4]> for Bind {
    fn from(arr: [f64; 4]) -> Self {
        Bind::TopRightBottomLeft(
            Unit::Number(arr[0]),
            Unit::Number(arr[1]),
            Unit::Number(arr[2]),
            Unit::Number(arr[3]),
        )
    }
}

pub enum EvalResult {
    Number(f64),
    Ref(String),
}

impl fmt::Display for EvalResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalResult::Number(num) => write!(f, "{}", num),
            EvalResult::Ref(expr) => write!(f, "{}", expr),
        }
    }
}

impl EvalResult {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            EvalResult::Number(num) => Some(*num),
            _ => None,
        }
    }

    pub fn resolve_as_number(&self, name: &str) -> Result<f64> {
        self.as_number().ok_or(Error::TypeError {
            field: name.to_owned(),
            expected: "number".to_owned(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Points {
    pub zones: IndexMap<String, Zone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror: Option<Mirror>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<Unit>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Asym {
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "clone")]
    Clone,
}

impl Asym {
    pub fn is_source(&self) -> bool {
        matches!(self, Asym::Source)
    }

    pub fn is_clone(&self) -> bool {
        matches!(self, Asym::Clone)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Key {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<Box<Zone>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<Box<Column>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_name: Option<String>,

    /// Column staggering means an extra vertical shift to the starting point of a whole column
    /// compared to the previous one (initially 0, cumulative afterwards). Its default value is 0
    /// (also overrideable with the $default_stagger internal variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stagger: Option<Unit>,

    /// Once a column has been laid out, spread (the horizontal space between this column and the
    /// next) is applied before the layout of the next column begins. Its default value is u (also
    /// overrideable with the $default_spread internal variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<Unit>,

    /// As a kind of companion to spread, splay applies a rotation (around an optional origin) to
    /// the starting point of a new column. Its default value is 0 (also overrideable with the
    /// $default_splay internal variable), and it rotates around the default origin of [0,
    /// 0] (meaning the center of where the first key in the column would go).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splay: Option<Unit>,

    /// Once a point within a column is determined, padding represents the vertical gap between it
    /// and the next row. Its default value is u (also overrideable with the $default_padding
    /// internal variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<Unit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<(Unit, Unit)>,

    /// The names might be familiar from the anchor section. And indeed, they do behave very
    /// similarly – only they are interpreted cumulatively within a column. The current key orients
    /// (default = 0), shifts (default = [0, 0]), and rotates (default = 0), and in doing so, not
    /// only positions itself, but provides the starting point for the next row within the column
    /// (to which the above padding can be applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orient: Option<Unit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift: Option<Shift>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<Unit>,

    /// This field is also used to adjust individual points – but, as opposed to the above trio,
    /// it's parsed as an actual anchor, and it applies independently, affecting only the current
    /// key and not the cumulative column layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjust: Option<Anchor>,

    /// Represents the amount of directional "reach" each key has when it tries to bind with its
    /// neighbors to form a contiguous shape. For a more in-depth explanation, check the outlines
    /// section. The value can be a number (uniform reach in every direction), an array of two
    /// numbers (horizontal/vertical reach), or an array of four numbers (top, right, bottom, and
    /// left reach, respectively – similarly to how CSS would assign things). The default is no
    /// bind (represented by -1, to differentiate from 0 length reaches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<Bind>,

    /// Enables automatically assigned binding in relevant direction to combine traditional
    /// keywells. For a more in-depth explanation, check the outlines section. Its default value is
    /// 10 (also overrideable with the $default_autobind internal variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autobind: Option<Unit>,

    /// This field signals that the current point is just a "helper" and should not be included in
    /// the output. This can happen when a real point is more easily calculable through a "stepping
    /// stone", but then we don't actually want the stepping stone to be a key itself. The default
    /// is, of course, false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<bool>,

    /// Determines which side of the keyboard the key should belong to (see Mirroring). Its default
    /// value is both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asym: Option<Asym>,

    /// Provides a way to override any key-level attributes for mirrored keys (see Mirroring).
    /// Empty by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror: Option<Mirror>,

    /// Built-in convenience variable to store a concatenated name of the column and the row,
    /// uniquely identifying a key within a zone. Its value is {{col.name}}_{{row}}, built through
    /// templating (see below).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colrow: Option<String>,

    /// The name of the key that identifies it uniquely not just within its zone, but globally. Its
    /// default value is {{zone.name}}_{{colrow}}, built through templating (see below). note
    /// Single key zones are common helpers for defining and naming interesting points on the
    /// board. To spare you from having to reference these as zonename_default_default (each
    /// default being the default column or row name, respectively, when nothing is specified),
    /// default suffices are always trimmed. So for single key zones, the name of the key is
    /// equivalent to the name of the zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// width / height: Helper values to signify the keycap width/height intended for the current
    /// position(s).
    ///
    /// Caution: These values only apply to the demo representation of the calculated key
    /// positions. For actual outlines to be cut (or used as a basis for cases), see the outlines
    /// section.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Unit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Unit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<IndexMap<String, serde_json::Value>>,
}

impl Key {
    pub fn new_default(units: &Units) -> Self {
        Self {
            stagger: units.get("$default_stagger").cloned(),
            spread: units.get("$default_spread").cloned(),
            splay: units.get("$default_splay").cloned(),
            origin: Some((Unit::Number(0.0), Unit::Number(0.0))),
            orient: Some(Unit::Number(0.0)),
            shift: Some(Shift::XY(Unit::Number(0.0), Unit::Number(0.0))),
            rotate: Some(Unit::Number(0.0)),
            adjust: None,
            width: units.get("$default_width").cloned(),
            height: units.get("$default_height").cloned(),
            padding: units.get("$default_padding").cloned(),
            autobind: units.get("$default_autobind").cloned(),
            skip: Some(false),
            asym: Some(Asym::Both),
            colrow: Some("{{col.name}}_{{row}}".to_string()),
            name: Some("{{zone.name}}_{{colrow}}".to_string()),
            ..Default::default()
        }
    }

    pub fn resolve(&self, units: &IndexMap<String, f64>) -> Result<ParsedKey> {
        let stagger = self
            .stagger
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("stagger", units).map(Some))?;
        let spread = self
            .spread
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("spread", units).map(Some))?;
        let splay = self
            .splay
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("splay", units).map(Some))?;
        let padding = self
            .padding
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("padding", units).map(Some))?;
        let origin = self.resolve_origin("origin", units).map(Some)?;
        let orient = self
            .orient
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("orient", units).map(Some))?;
        let shift = self
            .shift
            .as_ref()
            .map_or(Ok(None), |s| s.eval_as_numbers("shift", units).map(Some))?;
        let rotate = self
            .rotate
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("rotate", units).map(Some))?;
        let bind = self
            .bind
            .as_ref()
            .map_or(Ok(None), |b| b.resolve(units).map(Some))?;
        let autobind = self
            .autobind
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("autobind", units).map(Some))?;
        let skip = self.skip;
        let asym = self.asym;
        let width = self
            .width
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("width", units).map(Some))?;
        let height = self
            .height
            .as_ref()
            .map_or(Ok(None), |u| u.eval_as_number("height", units).map(Some))?;

        Ok(ParsedKey {
            stagger,
            spread,
            splay,
            padding,
            origin,
            orient,
            shift,
            rotate,
            bind,
            autobind,
            skip,
            asym,
            width,
            height,
            // name: todo!(),
            // zone: todo!(),
            // row: todo!(),
            // col_name: todo!(),
            // adjust: todo!(),
            // colrow: todo!(),
            // mirrored: todo!(),
            // affect: todo!(),
            // meta: todo!(),
            ..Default::default()
        })
    }

    pub fn process_templates(&self) -> Result<Key> {
        let key_obj = serde_json::to_value(self)?;
        let key_obj = key_obj.as_object().ok_or(Error::TypeError {
            field: "key".to_owned(),
            expected: "object".to_owned(),
        })?;

        let key_obj = process_templates(key_obj);

        Ok(serde_json::from_value(Value::Object(key_obj))?)
    }

    pub fn extend(&mut self, other: &Key) {
        if let Some(stagger) = &other.stagger {
            self.stagger = Some(stagger.clone());
        }
        if let Some(spread) = &other.spread {
            self.spread = Some(spread.clone());
        }
        if let Some(splay) = &other.splay {
            self.splay = Some(splay.clone());
        }
        if let Some(padding) = &other.padding {
            self.padding = Some(padding.clone());
        }
        if let Some(origin) = &other.origin {
            self.origin = Some(origin.clone());
        }
        if let Some(orient) = &other.orient {
            self.orient = Some(orient.clone());
        }
        if let Some(shift) = &other.shift {
            self.shift = Some(shift.clone());
        }
        if let Some(rotate) = &other.rotate {
            self.rotate = Some(rotate.clone());
        }
        if let Some(adjust) = &other.adjust {
            self.adjust = Some(adjust.clone());
        }
        if let Some(bind) = &other.bind {
            self.bind = Some(bind.clone());
        }
        if let Some(autobind) = &other.autobind {
            self.autobind = Some(autobind.clone());
        }
        if let Some(skip) = &other.skip {
            self.skip = Some(*skip);
        }
        if let Some(asym) = &other.asym {
            self.asym = Some(*asym);
        }
        if let Some(mirror) = &other.mirror {
            self.mirror = Some(mirror.clone());
        }
        if let Some(colrow) = &other.colrow {
            self.colrow = Some(colrow.clone());
        }
        if let Some(name) = &other.name {
            self.name = Some(name.clone());
        }
        if let Some(width) = &other.width {
            self.width = Some(width.clone());
        }
        if let Some(height) = &other.height {
            self.height = Some(height.clone());
        }

        // Handle the meta field specially
        if let Some(other_meta) = &other.meta {
            if self.meta.is_none() {
                // If self has no meta, just clone other's meta
                self.meta = Some(other_meta.clone());
            } else if let Some(self_meta) = &mut self.meta {
                // If both have meta, extend self's meta with other's meta
                for (key, value) in other_meta {
                    self_meta.insert(key.clone(), value.clone());
                }
            }
        }
    }

    pub fn resolve_origin(&self, arg: &str, units: &IndexMap<String, f64>) -> Result<(f64, f64)> {
        let (x, y) = self
            .origin
            .clone()
            .unwrap_or((Unit::Number(0.0), Unit::Number(0.0)));
        let x = x.eval_as_number(&format!("{}.origin.x", arg), units)?;
        let y = y.eval_as_number(&format!("{}.origin.y", arg), units)?;
        Ok((x, y))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mirror {
    pub ref_: Option<String>,
    pub distance: Option<Unit>,
    #[serde(flatten)]
    pub anchor: AnchorInfo,
}

impl Anchored for Mirror {
    fn ref_(&self) -> Option<Anchor> {
        self.ref_.as_ref().map(|ref_| Anchor::Ref(ref_.to_string()))
    }

    fn aggregate(&self) -> Option<Aggregate> {
        None
    }

    fn orient(&self) -> Option<Unit> {
        self.anchor.orient()
    }

    fn shift(&self) -> Option<Shift> {
        self.anchor.shift()
    }

    fn rotate(&self) -> Option<Unit> {
        self.anchor.rotate()
    }

    fn affect(&self) -> Option<Vec<AffectType>> {
        self.anchor.affect()
    }

    fn resist(&self) -> Option<bool> {
        self.anchor.resist()
    }

    fn asym(&self) -> Option<Asym> {
        self.anchor.asym()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outlines {}

// TODO: implement $unset
// #[derive(Clone, Debug, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum Row {
//     Unset,
//     Value(RowItem),
// }
//
// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct RowItem {}

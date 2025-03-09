use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{anchor::AnchorItem, types::Unit};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outline {
    Array(Vec<OutlineItem>),
    Object(IndexMap<String, OutlineItem>),
}

impl Outline {
    pub fn iter(&self) -> OutlineIter<'_> {
        self.into_iter()
    }
}

pub struct OutlineIter<'a> {
    inner: OutlineIterEnum<'a>,
}

enum OutlineIterEnum<'a> {
    Array {
        items: &'a Vec<OutlineItem>,
        index: usize,
    },
    Object {
        iter: indexmap::map::Iter<'a, String, OutlineItem>,
    },
}

pub enum OutlineKey<'a> {
    Index(usize),
    String(&'a String),
}

impl fmt::Display for OutlineKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlineKey::Index(index) => write!(f, "{}", index),
            OutlineKey::String(key) => write!(f, "{}", key),
        }
    }
}

impl<'a> IntoIterator for &'a Outline {
    type Item = (OutlineKey<'a>, &'a OutlineItem);
    type IntoIter = OutlineIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Outline::Array(items) => OutlineIter {
                inner: OutlineIterEnum::Array { items, index: 0 },
            },
            Outline::Object(map) => OutlineIter {
                inner: OutlineIterEnum::Object { iter: map.iter() },
            },
        }
    }
}

impl<'a> Iterator for OutlineIter<'a> {
    type Item = (OutlineKey<'a>, &'a OutlineItem);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            OutlineIterEnum::Array { items, index } => {
                if *index < items.len() {
                    let item = &items[*index];
                    let key = OutlineKey::Index(*index);
                    *index += 1;
                    Some((key, item))
                } else {
                    None
                }
            }
            OutlineIterEnum::Object { iter } => iter
                .next()
                .map(|(key, value)| (OutlineKey::String(key), value)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "what")]
pub enum OutlineItem {
    #[serde(rename = "circle")]
    Circle {
        #[serde(rename = "where")]
        where_: Option<Where>,
        radius: Unit,
    },
    #[serde(rename = "outline")]
    Outline {
        #[serde(rename = "where")]
        where_: Option<Where>,
        name: Option<String>,
        expand: Option<Unit>,
        fillet: Option<Unit>,
        joints: Option<Unit>,
    },
    #[serde(rename = "rectangle")]
    Rectangle {
        #[serde(rename = "where")]
        where_: Option<Where>,
        width: Option<String>,
        height: Option<String>,
        size: Size,
        corner: Option<Unit>,
        bevel: Option<Unit>,
    },
    #[serde(rename = "polygon")]
    Polygon {
        #[serde(rename = "where")]
        where_: Option<Where>,
        points: Vec<AnchorItem>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Size {
    Multi((Unit, Unit)),
    Single(Unit),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Where {
    Bool(bool),
    String(String),
    Array(Vec<Where>),
}

#[cfg(test)]
mod tests {
    use crate::anchor::{Anchor, AnchorItem};

    use super::*;

    #[test]
    fn test_parse_outline() {
        let outline = r#"
screws:
  - what: circle
    where: /screw_pcb/
    radius: screw_radius

bottom_case_outer_outline:
  - what: outline
    name: _backplate_outline
    expand: case_wall_thickness + pcb_to_case_wall_tolerance
    fillet: 0.5
    joints: 1

mcu_wall_cutout:
- what: rectangle
  where: matrix_inner_top
  size: [8, 40 + pcb_to_case_wall_tolerance * 2 + case_wall_thickness * 2]
  adjust:
    shift:
      [
        19.704 - 1.25 + pcb_to_case_wall_tolerance / 2 + case_wall_thickness,
        0,
      ]

backplate_additional_outline:
  - what: polygon
    points:
      - ref: mcu_cover_top_left
      - ref: mcu_cover_top_right
      - ref: mcu_cover_bottom_right
      - ref: mcu_cover_bottom_left
"#;

        let outline: IndexMap<String, Outline> = serde_yaml::from_str(outline).unwrap();
        println!("{:#?}", outline);
    }

    #[test]
    fn test_serialize_polygon() {
        let polygon = OutlineItem::Polygon {
            where_: None,
            points: vec![AnchorItem {
                ref_: Some(Anchor::Ref("mcu_cover_top_left".to_string())),
                ..Default::default()
            }],
        };

        let serialized = serde_yaml::to_string(&polygon).unwrap();
        println!("{}", serialized);
    }
}

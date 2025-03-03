use nalgebra::{Point2, Rotation2, Vector2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;

use crate::utils;

/// A point in 2D space with rotation and metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64, r: f64, meta: HashMap<String, serde_json::Value>) -> Self {
        Self { x, y, r, meta }
    }

    /// Create a default point at the origin
    pub fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            r: 0.0,
            meta: HashMap::new(),
        }
    }

    /// Create a point from an array [x, y]
    pub fn from_array(arr: &[f64]) -> Self {
        if arr.len() >= 2 {
            Self {
                x: arr[0],
                y: arr[1],
                r: 0.0,
                meta: HashMap::new(),
            }
        } else {
            Self::default()
        }
    }

    /// Get the point as an array [x, y]
    pub fn p(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    /// Set the point from an array [x, y]
    pub fn set_p(&mut self, val: [f64; 2]) {
        self.x = val[0];
        self.y = val[1];
    }

    /// Shift the point by the given amount
    pub fn shift(&mut self, s: [f64; 2], relative: bool, resist: bool) -> &mut Self {
        let mut shift = s;

        // If mirrored and not resisting, negate the x shift
        if !resist {
            if let Some(mirrored) = self.meta.get("mirrored") {
                if mirrored.as_bool().unwrap_or(false) {
                    shift[0] *= -1.0;
                }
            }
        }

        // If relative, rotate the shift by the point's rotation
        if relative {
            let rotation = Rotation2::new(self.r * PI / 180.0);
            let shift_vec = Vector2::new(shift[0], shift[1]);
            let rotated = rotation * shift_vec;
            shift = [rotated.x, rotated.y];
        }

        self.x += shift[0];
        self.y += shift[1];

        self
    }

    /// Rotate the point by the given angle
    pub fn rotate(&mut self, angle: f64, origin: Option<[f64; 2]>, resist: bool) -> &mut Self {
        let mut adjusted_angle = angle;

        // If mirrored and not resisting, negate the angle
        if !resist {
            if let Some(mirrored) = self.meta.get("mirrored") {
                if mirrored.as_bool().unwrap_or(false) {
                    adjusted_angle *= -1.0;
                }
            }
        }

        // If an origin is provided, rotate around that point
        if let Some(origin) = origin {
            let origin_point = Point2::new(origin[0], origin[1]);
            let mut point = Point2::new(self.x, self.y);

            let rotation = Rotation2::new(adjusted_angle * PI / 180.0);
            let translated = point - origin_point.coords;
            let rotated = rotation * translated;
            point = Point2::new(origin_point.x + rotated.x, origin_point.y + rotated.y);

            self.x = point.x;
            self.y = point.y;
        }

        self.r += adjusted_angle;

        self
    }

    /// Mirror the point around the x-coordinate
    pub fn mirror(&mut self, x: f64) -> &mut Self {
        self.x = 2.0 * x - self.x;
        self.r = -self.r;
        self
    }

    /// Create a deep clone of this point
    pub fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            r: self.r,
            meta: utils::deepcopy(&self.meta),
        }
    }

    /// Position a model relative to this point
    pub fn position<T>(&self, model: T) -> T {
        // In a full implementation, this would apply rotation and translation
        // to the model based on this point's position and rotation
        // For now, we just return the model
        model
    }

    /// Remove the position of this point from a model
    pub fn unposition<T>(&self, model: T) -> T {
        // In a full implementation, this would remove this point's rotation and translation
        // from the model
        // For now, we just return the model
        model
    }

    /// Create a rectangle centered at this point
    pub fn rect(&self, size: f64) -> utils::Rect {
        let half_size = size / 2.0;
        let rect = utils::rect(size, size, Some([-half_size, -half_size]));

        // In a full implementation, this would position the rect at this point
        rect
    }

    /// Calculate the angle to another point
    pub fn angle(&self, other: &Self) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        -dy.atan2(dx) * 180.0 / PI
    }

    /// Check if this point equals another point
    pub fn equals(&self, other: &Self) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.r == other.r
            && match (
                serde_json::to_string(&self.meta),
                serde_json::to_string(&other.meta),
            ) {
                (Ok(self_meta), Ok(other_meta)) => self_meta == other_meta,
                _ => false, // If serialization fails, consider them not equal
            }
    }
}

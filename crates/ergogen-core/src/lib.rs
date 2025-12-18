use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    #[serde(default)]
    pub meta: PointMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PointMeta {
    pub name: String,
    pub zone: String,
    pub column: String,
    pub row: String,
    #[serde(default = "default_dimension")]
    pub width: f64,
    #[serde(default = "default_dimension")]
    pub height: f64,
    #[serde(default)]
    pub mirrored: bool,
    #[serde(default)]
    pub bind: [f64; 4],
}

fn default_dimension() -> f64 {
    1.0
}

impl Point {
    pub fn new(x: f64, y: f64, r: f64) -> Self {
        Self {
            x,
            y,
            r,
            meta: PointMeta::default(),
        }
    }

    pub fn shift(&mut self, delta: [f64; 2]) {
        let rad = self.r * PI / 180.0;
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        self.x += delta[0] * cos_r - delta[1] * sin_r;
        self.y += delta[0] * sin_r + delta[1] * cos_r;
    }

    pub fn rotate(&mut self, angle: f64, origin: Option<[f64; 2]>) {
        let origin = origin.unwrap_or([self.x, self.y]);
        let rad = angle * PI / 180.0;
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        let dx = self.x - origin[0];
        let dy = self.y - origin[1];

        self.x = origin[0] + dx * cos_a - dy * sin_a;
        self.y = origin[1] + dx * sin_a + dy * cos_a;
        self.r += angle;
    }

    pub fn mirror(&mut self, axis: f64) {
        self.x = 2.0 * axis - self.x;
        self.r = 180.0 - self.r;
        self.meta.mirrored = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_shift() {
        let mut p = Point::new(0.0, 0.0, 0.0);
        p.shift([10.0, 5.0]);
        assert_relative_eq!(p.x, 10.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_shift_with_rotation() {
        let mut p = Point::new(0.0, 0.0, 90.0);
        p.shift([10.0, 0.0]);
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rotate() {
        let mut p = Point::new(10.0, 0.0, 0.0);
        p.rotate(90.0, Some([0.0, 0.0]));
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-10);
        assert_relative_eq!(p.y, 10.0, epsilon = 1e-10);
        assert_relative_eq!(p.r, 90.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mirror() {
        let mut p = Point::new(10.0, 5.0, 45.0);
        p.mirror(0.0);
        assert_relative_eq!(p.x, -10.0, epsilon = 1e-10);
        assert_relative_eq!(p.r, 135.0, epsilon = 1e-10);
        assert!(p.meta.mirrored);
    }
}

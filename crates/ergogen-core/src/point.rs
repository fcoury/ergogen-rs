use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    /// Rotation in degrees.
    pub r: f64,
    pub meta: PointMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PointMeta {
    pub mirrored: bool,
}

impl Point {
    #[must_use]
    pub fn new(x: f64, y: f64, r: f64, meta: PointMeta) -> Self {
        Self { x, y, r, meta }
    }

    #[must_use]
    pub fn xy(x: f64, y: f64) -> Self {
        Self::new(x, y, 0.0, PointMeta::default())
    }

    /// Shift this point by `shift`.
    ///
    /// Behavior matches Ergogen's JS Point:
    /// - When `relative` is true, `shift` is rotated by the point's current rotation.
    /// - When the point is mirrored and `resist` is false, the X component is flipped.
    pub fn shift(&mut self, mut shift: [f64; 2], relative: bool, resist: bool) -> &mut Self {
        if self.meta.mirrored && !resist {
            shift[0] *= -1.0;
        }
        if relative {
            shift = rotate_vec(shift, self.r);
        }
        self.x += shift[0];
        self.y += shift[1];
        self
    }

    /// Rotate this point by `angle_deg` around `origin` (when provided) and update its rotation.
    ///
    /// When the point is mirrored and `resist` is false, the angle is negated.
    pub fn rotate(
        &mut self,
        mut angle_deg: f64,
        origin: Option<[f64; 2]>,
        resist: bool,
    ) -> &mut Self {
        if self.meta.mirrored && !resist {
            angle_deg *= -1.0;
        }
        if let Some(origin) = origin {
            let p = rotate_point([self.x, self.y], angle_deg, origin);
            self.x = p[0];
            self.y = p[1];
        }
        self.r += angle_deg;
        self
    }

    /// Mirror this point across the vertical line `x = axis_x`.
    pub fn mirror(&mut self, axis_x: f64) -> &mut Self {
        self.x = 2.0 * axis_x - self.x;
        self.r = -self.r;
        self
    }

    /// Angle (degrees) from this point to `other`, using Ergogen's convention.
    #[must_use]
    pub fn angle_to(&self, other: &Point) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        -(dx).atan2(dy).to_degrees()
    }
}

fn rotate_vec(v: [f64; 2], angle_deg: f64) -> [f64; 2] {
    let a = angle_deg.to_radians();
    let (s, c) = a.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

fn rotate_point(p: [f64; 2], angle_deg: f64, origin: [f64; 2]) -> [f64; 2] {
    let translated = [p[0] - origin[0], p[1] - origin[1]];
    let rotated = rotate_vec(translated, angle_deg);
    [rotated[0] + origin[0], rotated[1] + origin[1]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn shift_relative_rotates_vector() {
        let mut p = Point::xy(0.0, 0.0);
        p.r = 90.0;
        p.shift([1.0, 0.0], true, false);
        assert_abs_diff_eq!(p.x, 0.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.y, 1.0, epsilon = 1e-9);
    }

    #[test]
    fn shift_mirrored_flips_x_before_rotation() {
        let mut p = Point::new(0.0, 0.0, 90.0, PointMeta { mirrored: true });
        p.shift([1.0, 0.0], true, false);
        assert_abs_diff_eq!(p.x, 0.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.y, -1.0, epsilon = 1e-9);
    }

    #[test]
    fn rotate_about_origin_updates_position_and_r() {
        let mut p = Point::xy(1.0, 0.0);
        p.rotate(90.0, Some([0.0, 0.0]), false);
        assert_abs_diff_eq!(p.x, 0.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.y, 1.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.r, 90.0, epsilon = 1e-9);
    }

    #[test]
    fn mirror_flips_x_and_negates_r() {
        let mut p = Point::new(10.0, 5.0, 30.0, PointMeta::default());
        p.mirror(7.0);
        assert_abs_diff_eq!(p.x, 4.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.y, 5.0, epsilon = 1e-9);
        assert_abs_diff_eq!(p.r, -30.0, epsilon = 1e-9);
    }
}

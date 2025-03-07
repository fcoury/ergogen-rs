use crate::{point, schema::Point};

pub fn point_in_radians(origin: Point, angle_point: Point) -> f64 {
    let d = point::subtract(angle_point, origin);
    let x = d.0;
    let y = d.1;

    -y.atan2(-x) + std::f64::consts::PI
}

pub fn to_radians(angle: f64) -> f64 {
    angle.to_radians()
}

pub fn to_degrees(angle: f64) -> f64 {
    angle.to_degrees()
}

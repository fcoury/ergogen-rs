use super::zone::Point;

pub fn average(points: &[Point]) -> Point {
    if points.len() == 0 {
        return Point::default();
    }

    let mut x = 0.0;
    let mut y = 0.0;
    let mut r = 0.0;

    for point in points {
        x += point.x.unwrap_or_default();
        y += point.y.unwrap_or_default();
        r += point.r.unwrap_or_default();
    }

    Point {
        x: Some(x / points.len() as f64),
        y: Some(y / points.len() as f64),
        r: Some(r / points.len() as f64),
        ..Default::default()
    }
}

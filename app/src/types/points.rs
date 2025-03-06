use super::zone::Point;

#[derive(Clone, Debug)]
pub struct Rotation {
    pub angle: f64,
    pub origin: Point,
}

pub fn apply_rotations(rotations: &[Rotation], angle: f64, origin: &Point) -> Rotation {
    let mut candidate = origin.clone();

    for r in rotations.iter() {
        candidate = candidate.rotated(r.angle, Some(r.origin.p()), false);
    }

    Rotation {
        angle,
        origin: candidate.clone(),
    }
}

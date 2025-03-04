use super::zone::Point;

#[derive(Clone, Debug)]
pub struct Rotation {
    pub angle: f64,
    pub origin: Point,
}

pub fn apply_rotations(rotations: &[Rotation], angle: f64, origin: &Point) -> Rotation {
    let mut candidate = origin;

    for r in rotations.iter() {
        // TODO: candidate = makerjs.point.rotate(candidate, r.angle, r.origin);
        todo!("waiting for maker-rs");
    }

    Rotation {
        angle,
        origin: candidate.clone(),
    }
}

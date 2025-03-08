#[derive(Clone, Debug)]
pub struct Rotation {
    pub angle: f64,
    pub origin: (f64, f64),
}

pub fn apply_rotations(rotations: &[Rotation], angle: f64, origin: (f64, f64)) -> Rotation {
    let mut candidate = origin;

    for r in rotations.iter() {
        candidate = maker_rs::point::rotate(candidate, r.angle, Some(r.origin));
    }

    Rotation {
        angle,
        origin: candidate,
    }
}

pub mod angle;
pub mod measure;
pub mod path;
pub mod paths;
pub mod point;
pub mod schema;

pub struct Slope {
    pub has_slope: bool,
    pub slope: f64,
    pub line: paths::Line,
    pub y_intercept: f64,
}

impl Slope {
    pub fn is_parallel(&self, other: &Slope) -> bool {
        if !self.has_slope && !other.has_slope {
            return true;
        }

        if self.has_slope && other.has_slope {
            return (self.slope - other.slope).abs() < f64::EPSILON;
        }

        false
    }
}

impl PartialEq for Slope {
    fn eq(&self, other: &Self) -> bool {
        if !self.is_parallel(other) {
            return false;
        }

        if !self.has_slope && !other.has_slope {
            return self.line.origin.0 - other.line.origin.0 == 0.0;
        }

        let angles = [self, other]
            .iter()
            .map(|s| angle::to_degrees(s.slope.atan()))
            .collect::<Vec<_>>();

        let mut lines = [self, other]
            .iter()
            .map(|s| s.line.clone())
            .collect::<Vec<_>>();

        lines[0] = path::rotate(&lines[0], -angles[0], Some(lines[0].origin));
        lines[1] = path::rotate(&lines[1], -angles[1], Some(lines[1].origin));

        let average_ys = lines
            .iter()
            .map(|line| (line.origin.1 + line.end.1) / 2.0)
            .collect::<Vec<_>>();

        (average_ys[0] - average_ys[1]).abs() < f64::EPSILON
    }
}

use crate::schema::Point;

pub enum PathType {
    Line,
}

pub trait Path {
    fn typ_(&self) -> PathType;

    fn origin(&self) -> (f64, f64);

    fn set_origin(&mut self, origin: (f64, f64));

    fn set_end(&mut self, end: (f64, f64));

    fn layer(&self) -> Option<String> {
        None
    }

    fn as_line(&self) -> Option<&Line> {
        None
    }
}

pub trait PathLine: Path {
    fn end(&self) -> (f64, f64);

    fn as_line(&self) -> Option<&Line> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub origin: (f64, f64),
    pub end: (f64, f64),
}

impl From<Point> for Line {
    fn from(point: Point) -> Self {
        Line {
            origin: (point.0, point.1),
            end: (point.0, point.1),
        }
    }
}

impl From<Line> for Point {
    fn from(line: Line) -> Self {
        line.origin
    }
}

impl Path for Line {
    fn typ_(&self) -> PathType {
        PathType::Line
    }

    fn origin(&self) -> (f64, f64) {
        self.origin
    }

    fn set_origin(&mut self, origin: (f64, f64)) {
        self.origin = origin;
    }

    fn set_end(&mut self, end: (f64, f64)) {
        self.end = end;
    }

    fn as_line(&self) -> Option<&Line> {
        Some(self)
    }
}

impl PathLine for Line {
    fn end(&self) -> (f64, f64) {
        self.end
    }
}

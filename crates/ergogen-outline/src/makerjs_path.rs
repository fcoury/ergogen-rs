use std::f64::consts::PI;

use ergogen_geometry::{PlineVertex, Polyline};

#[derive(Debug, thiserror::Error)]
pub enum MakerJsPathError {
    #[error("invalid arc (3 points are collinear)")]
    ArcFrom3PointsCollinear,
    #[error("path does not form a single closed chain")]
    NotClosedChain,
    #[error("path chain is disconnected at {at:?}")]
    Disconnected {
        at: [f64; 2],
        segment_index: usize,
        segment_a: [f64; 2],
        segment_b: [f64; 2],
    },
}

#[derive(Debug, Clone, Copy)]
pub struct MakerArc {
    pub origin: [f64; 2],
    pub radius: f64,
    pub start_angle_deg: f64,
    pub end_angle_deg: f64,
}

#[derive(Debug, Clone)]
pub enum Primitive {
    Line { a: [f64; 2], b: [f64; 2] },
    Arc {
        arc: MakerArc,
        a: [f64; 2],
        b: [f64; 2],
        reversed: bool,
    },
}

impl Primitive {
    pub fn endpoints(&self) -> ([f64; 2], [f64; 2]) {
        match self {
            Primitive::Line { a, b } => (*a, *b),
            Primitive::Arc { a, b, reversed, .. } => {
                if *reversed { (*b, *a) } else { (*a, *b) }
            }
        }
    }

    pub fn reversed(&self) -> Self {
        match self {
            Primitive::Line { a, b } => Primitive::Line { a: *b, b: *a },
            Primitive::Arc {
                arc,
                a,
                b,
                reversed,
            } => Primitive::Arc {
                arc: *arc,
                a: *a,
                b: *b,
                reversed: !reversed,
            },
        }
    }
}

fn makerjs_round(n: f64, accuracy: f64) -> f64 {
    if n.fract() == 0.0 {
        return n;
    }
    let temp = 1.0 / accuracy;
    ((n + f64::EPSILON) * temp).round() / temp
}

fn points_equal_xy_round(a: [f64; 2], b: [f64; 2]) -> bool {
    makerjs_round(a[0] - b[0], 1e-7) == 0.0 && makerjs_round(a[1] - b[1], 1e-7) == 0.0
}

fn points_equal_xy_within(a: [f64; 2], b: [f64; 2], within: f64) -> bool {
    point_distance(a, b) <= within
}

fn angle_of_point_in_degrees(origin: [f64; 2], p: [f64; 2]) -> f64 {
    // MakerJs.angle.ofPointInRadians: atan2(-y, -x) + PI
    let dx = p[0] - origin[0];
    let dy = p[1] - origin[1];
    let rad = (-dy).atan2(-dx) + PI;
    rad * 180.0 / PI
}

fn no_revolutions(angle_deg: f64) -> f64 {
    let revolutions = (angle_deg / 360.0).floor();
    if revolutions == 0.0 {
        return angle_deg;
    }
    angle_deg - 360.0 * revolutions
}

fn arc_end_angle_deg(arc: MakerArc) -> f64 {
    if arc.end_angle_deg < arc.start_angle_deg {
        let mut end = arc.end_angle_deg;
        while end < arc.start_angle_deg {
            end += 360.0;
        }
        end
    } else {
        arc.end_angle_deg
    }
}

fn arc_span_deg(arc: MakerArc) -> f64 {
    // MakerJs.angle.ofArcSpan
    let end = arc_end_angle_deg(arc);
    let span = end - arc.start_angle_deg;
    if makerjs_round(span, 1e-7) > 360.0 {
        no_revolutions(span)
    } else {
        span
    }
}

fn is_between(value: f64, a: f64, b: f64, exclusive: bool) -> bool {
    if exclusive {
        a.min(b) < value && value < a.max(b)
    } else {
        a.min(b) <= value && value <= a.max(b)
    }
}

fn is_between_arc_angles(angle: f64, arc: MakerArc, exclusive: bool) -> bool {
    // MakerJs.measure.isBetweenArcAngles
    let start = no_revolutions(arc.start_angle_deg);
    let end = start + arc_span_deg(arc);
    let angle = no_revolutions(angle);
    is_between(angle, start, end, exclusive)
        || is_between(angle, start + 360.0, end + 360.0, exclusive)
        || is_between(angle, start - 360.0, end - 360.0, exclusive)
}

fn point_from_polar(angle_rad: f64, radius: f64) -> [f64; 2] {
    let (s, c) = angle_rad.sin_cos();
    [
        makerjs_round(radius * c, 1e-7),
        makerjs_round(radius * s, 1e-7),
    ]
}

fn arc_point(arc: MakerArc, angle_deg: f64) -> [f64; 2] {
    let rad = no_revolutions(angle_deg) * PI / 180.0;
    let p = point_from_polar(rad, arc.radius);
    [arc.origin[0] + p[0], arc.origin[1] + p[1]]
}

pub fn arc_endpoints(arc: MakerArc) -> ([f64; 2], [f64; 2]) {
    (arc_point(arc, arc.start_angle_deg), arc_point(arc, arc.end_angle_deg))
}

fn arc_point_at_ratio(arc: MakerArc, ratio: f64) -> [f64; 2] {
    let a = arc.start_angle_deg + arc_span_deg(arc) * ratio;
    arc_point(arc, a)
}

fn point_distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    (dx * dx + dy * dy).sqrt()
}

pub fn arc_from_3_points(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Result<MakerArc, MakerJsPathError> {
    // Faithful port of MakerJS "Circle from 3 points":
    // - build 2 lines sharing the middle point
    // - rotate each by 90deg around its midpoint (MakerJS rotation uses rounded polar)
    // - intersect their slopes (with MakerJS' vertical / parallel rules)
    #[derive(Clone, Copy)]
    struct Line2 {
        origin: [f64; 2],
        end: [f64; 2],
    }

    fn midpoint(line: Line2) -> [f64; 2] {
        [(line.origin[0] + line.end[0]) / 2.0, (line.origin[1] + line.end[1]) / 2.0]
    }

    fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
        [a[0] + b[0], a[1] + b[1]]
    }

    fn from_polar(angle_rad: f64, radius: f64) -> [f64; 2] {
        // MakerJs.point.fromPolar: rounds trig results to 1e-7 and has exact-zero fast paths.
        let (s, c) = angle_rad.sin_cos();
        let x = if (angle_rad - PI / 2.0).abs() == 0.0 || (angle_rad - 3.0 * PI / 2.0).abs() == 0.0 {
            0.0
        } else {
            makerjs_round(radius * c, 1e-7)
        };
        let y = if (angle_rad - PI).abs() == 0.0 || (angle_rad - 2.0 * PI).abs() == 0.0 {
            0.0
        } else {
            makerjs_round(radius * s, 1e-7)
        };
        [x, y]
    }

    fn angle_of_point_in_radians(origin: [f64; 2], p: [f64; 2]) -> f64 {
        let dx = p[0] - origin[0];
        let dy = p[1] - origin[1];
        (-dy).atan2(-dx) + PI
    }

    fn rotate_point(p: [f64; 2], angle_deg: f64, rotation_origin: [f64; 2]) -> [f64; 2] {
        // MakerJs.point.rotate
        let point_angle = angle_of_point_in_radians(rotation_origin, p);
        let d = point_distance(rotation_origin, p);
        let rotated = from_polar(point_angle + no_revolutions(angle_deg) * PI / 180.0, d);
        add(rotation_origin, rotated)
    }

    fn rotate_line(mut line: Line2, angle_deg: f64, rotation_origin: [f64; 2]) -> Line2 {
        line.origin = rotate_point(line.origin, angle_deg, rotation_origin);
        line.end = rotate_point(line.end, angle_deg, rotation_origin);
        line
    }

    #[derive(Clone, Copy)]
    struct Slope {
        has_slope: bool,
        slope: f64,
        y_intercept: f64,
        vertical_x: f64,
    }

    fn line_slope(line: Line2) -> Slope {
        // MakerJs.measure.lineSlope
        let dx = line.end[0] - line.origin[0];
        if makerjs_round(dx, 1e-6) == 0.0 {
            return Slope {
                has_slope: false,
                slope: f64::NAN,
                y_intercept: f64::NAN,
                vertical_x: line.origin[0],
            };
        }
        let dy = line.end[1] - line.origin[1];
        let slope = dy / dx;
        let y_intercept = line.origin[1] - slope * line.origin[0];
        Slope {
            has_slope: true,
            slope,
            y_intercept,
            vertical_x: f64::NAN,
        }
    }

    fn slope_parallel(a: Slope, b: Slope) -> bool {
        // MakerJs.measure.isSlopeParallel
        if !a.has_slope && !b.has_slope {
            return true;
        }
        a.has_slope && b.has_slope && makerjs_round(a.slope - b.slope, 1e-5) == 0.0
    }

    fn slope_intersection(line_a: Line2, line_b: Line2) -> Option<[f64; 2]> {
        // MakerJs.point.fromSlopeIntersection (minimal: only returns null for parallel slopes).
        let sa = line_slope(line_a);
        let sb = line_slope(line_b);

        if slope_parallel(sa, sb) {
            return None;
        }

        if !sa.has_slope {
            let x = sa.vertical_x;
            let y = sb.slope * x + sb.y_intercept;
            return Some([x, y]);
        }
        if !sb.has_slope {
            let x = sb.vertical_x;
            let y = sa.slope * x + sa.y_intercept;
            return Some([x, y]);
        }

        let x = (sb.y_intercept - sa.y_intercept) / (sa.slope - sb.slope);
        let y = sa.slope * x + sa.y_intercept;
        Some([x, y])
    }

    let l1 = Line2 { origin: a, end: b };
    let l2 = Line2 { origin: b, end: c };
    let p1 = rotate_line(l1, 90.0, midpoint(l1));
    let p2 = rotate_line(l2, 90.0, midpoint(l2));

    let origin = slope_intersection(p1, p2).ok_or(MakerJsPathError::ArcFrom3PointsCollinear)?;
    let radius = point_distance(origin, a);

    let angles = [
        angle_of_point_in_degrees(origin, a),
        angle_of_point_in_degrees(origin, b),
        angle_of_point_in_degrees(origin, c),
    ];
    let mut arc = MakerArc {
        origin,
        radius,
        start_angle_deg: angles[0],
        end_angle_deg: angles[2],
    };
    if !is_between_arc_angles(angles[1], arc, false) {
        arc.start_angle_deg = angles[2];
        arc.end_angle_deg = angles[0];
    }
    Ok(arc)
}

#[derive(Debug, Clone)]
pub struct BezierSeed {
    pub origin: [f64; 2],
    pub controls: Vec<[f64; 2]>,
    pub end: [f64; 2],
}

impl BezierSeed {
    pub fn order(&self) -> usize {
        if self.controls.len() == 1 { 2 } else { 3 }
    }

    pub fn points(&self) -> Vec<[f64; 2]> {
        let mut pts = Vec::with_capacity(2 + self.controls.len());
        pts.push(self.origin);
        pts.extend(self.controls.iter().copied());
        pts.push(self.end);
        pts
    }
}

#[derive(Debug, Clone, Copy)]
struct TPoint {
    t: f64,
    point: [f64; 2],
}

#[derive(Debug, Clone)]
enum ArcOrLine {
    Arc { arc: MakerArc, end_t: f64 },
    Line { a: [f64; 2], b: [f64; 2], end_t: f64 },
}

fn bezier_compute(seed: &BezierSeed, t: f64) -> [f64; 2] {
    let pts = seed.points();
    let order = pts.len() - 1;
    if t == 0.0 {
        return pts[0];
    }
    if t == 1.0 {
        return pts[order];
    }
    let mt = 1.0 - t;
    if order == 1 {
        return [
            mt * pts[0][0] + t * pts[1][0],
            mt * pts[0][1] + t * pts[1][1],
        ];
    }
    if order == 2 {
        let mt2 = mt * mt;
        let t2 = t * t;
        let a = mt2;
        let b = mt * t * 2.0;
        let c = t2;
        return [
            a * pts[0][0] + b * pts[1][0] + c * pts[2][0],
            a * pts[0][1] + b * pts[1][1] + c * pts[2][1],
        ];
    }
    // cubic
    let mt2 = mt * mt;
    let t2 = t * t;
    let a = mt2 * mt;
    let b = mt2 * t * 3.0;
    let c = mt * t2 * 3.0;
    let d = t * t2;
    [
        a * pts[0][0] + b * pts[1][0] + c * pts[2][0] + d * pts[3][0],
        a * pts[0][1] + b * pts[1][1] + c * pts[2][1] + d * pts[3][1],
    ]
}

fn bezier_derivative(seed: &BezierSeed, t: f64) -> [f64; 2] {
    let pts = seed.points();
    let order = pts.len() - 1;
    let mt = 1.0 - t;
    if order == 2 {
        let d0 = [2.0 * (pts[1][0] - pts[0][0]), 2.0 * (pts[1][1] - pts[0][1])];
        let d1 = [2.0 * (pts[2][0] - pts[1][0]), 2.0 * (pts[2][1] - pts[1][1])];
        return [mt * d0[0] + t * d1[0], mt * d0[1] + t * d1[1]];
    }
    let d0 = [3.0 * (pts[1][0] - pts[0][0]), 3.0 * (pts[1][1] - pts[0][1])];
    let d1 = [3.0 * (pts[2][0] - pts[1][0]), 3.0 * (pts[2][1] - pts[1][1])];
    let d2 = [3.0 * (pts[3][0] - pts[2][0]), 3.0 * (pts[3][1] - pts[2][1])];
    let a = mt * mt;
    let b = mt * t * 2.0;
    let c = t * t;
    [a * d0[0] + b * d1[0] + c * d2[0], a * d0[1] + b * d1[1] + c * d2[1]]
}

fn bezier_length(seed: &BezierSeed) -> f64 {
    // bezier-js utils.length with Gauss-Legendre n=24.
    const TV: [f64; 24] = [
        -0.064_056_892_862_605_626_085_043_082_624_745_038_5909,
        0.064_056_892_862_605_626_085_043_082_624_745_038_5909,
        -0.191_118_867_473_616_309_158_639_820_757_069_631_8404,
        0.191_118_867_473_616_309_158_639_820_757_069_631_8404,
        -0.315_042_679_696_163_374_386_793_291_319_810_240_7864,
        0.315_042_679_696_163_374_386_793_291_319_810_240_7864,
        -0.433_793_507_626_045_138_487_084_231_913_349_712_4524,
        0.433_793_507_626_045_138_487_084_231_913_349_712_4524,
        -0.545_421_471_388_839_535_658_375_617_218_372_370_0107,
        0.545_421_471_388_839_535_658_375_617_218_372_370_0107,
        -0.648_093_651_936_975_569_252_495_786_910_747_626_6696,
        0.648_093_651_936_975_569_252_495_786_910_747_626_6696,
        -0.740_124_191_578_554_364_243_828_103_099_978_425_5232,
        0.740_124_191_578_554_364_243_828_103_099_978_425_5232,
        -0.820_001_985_973_902_921_953_949_872_669_745_208_0761,
        0.820_001_985_973_902_921_953_949_872_669_745_208_0761,
        -0.886_415_527_004_401_034_213_154_341_982_196_755_0873,
        0.886_415_527_004_401_034_213_154_341_982_196_755_0873,
        -0.938_274_552_002_732_758_523_649_001_708_721_449_6548,
        0.938_274_552_002_732_758_523_649_001_708_721_449_6548,
        -0.974_728_555_971_309_498_198_391_993_008_169_061_7411,
        0.974_728_555_971_309_498_198_391_993_008_169_061_7411,
        -0.995_187_219_997_021_360_179_997_409_700_736_811_8745,
        0.995_187_219_997_021_360_179_997_409_700_736_811_8745,
    ];
    const CV: [f64; 24] = [
        0.127_938_195_346_752_156_974_056_165_224_695_371_8517,
        0.127_938_195_346_752_156_974_056_165_224_695_371_8517,
        0.125_837_456_346_828_296_121_375_382_511_183_688_7264,
        0.125_837_456_346_828_296_121_375_382_511_183_688_7264,
        0.121_670_472_927_803_391_204_463_153_476_262_425_6070,
        0.121_670_472_927_803_391_204_463_153_476_262_425_6070,
        0.115_505_668_053_725_601_353_344_483_906_783_559_8622,
        0.115_505_668_053_725_601_353_344_483_906_783_559_8622,
        0.107_444_270_115_965_634_782_577_342_446_606_222_7946,
        0.107_444_270_115_965_634_782_577_342_446_606_222_7946,
        0.097_618_652_104_113_888_269_880_664_464_247_154_4279,
        0.097_618_652_104_113_888_269_880_664_464_247_154_4279,
        0.086_190_161_531_953_275_917_185_202_983_742_667_1850,
        0.086_190_161_531_953_275_917_185_202_983_742_667_1850,
        0.073_346_481_411_080_305_734_033_615_253_116_518_1193,
        0.073_346_481_411_080_305_734_033_615_253_116_518_1193,
        0.059_298_584_915_436_780_746_367_758_500_108_584_5412,
        0.059_298_584_915_436_780_746_367_758_500_108_584_5412,
        0.044_277_438_817_419_806_168_602_748_211_338_228_8593,
        0.044_277_438_817_419_806_168_602_748_211_338_228_8593,
        0.028_531_388_628_933_663_181_307_815_951_878_286_4491,
        0.028_531_388_628_933_663_181_307_815_951_878_286_4491,
        0.012_341_229_799_987_199_546_805_667_070_037_291_5759,
        0.012_341_229_799_987_199_546_805_667_070_037_291_5759,
    ];
    let z = 0.5;
    let mut sum = 0.0;
    for i in 0..TV.len() {
        let t = z * TV[i] + z;
        let d = bezier_derivative(seed, t);
        sum += CV[i] * (d[0] * d[0] + d[1] * d[1]).sqrt();
    }
    z * sum
}

fn droots(p: &[f64]) -> Vec<f64> {
    if p.len() == 3 {
        let a = p[0];
        let b = p[1];
        let c = p[2];
        let d = a - 2.0 * b + c;
        if d != 0.0 {
            let m1 = -(b * b - a * c).sqrt();
            let m2 = -a + b;
            let v1 = -(m1 + m2) / d;
            let v2 = -(-m1 + m2) / d;
            return vec![v1, v2];
        } else if b != c && d == 0.0 {
            return vec![(2.0 * b - c) / (2.0 * (b - c))];
        }
        return Vec::new();
    }
    if p.len() == 2 {
        let a = p[0];
        let b = p[1];
        if a != b {
            return vec![a / (a - b)];
        }
        return Vec::new();
    }
    Vec::new()
}

fn bezier_extrema_values(seed: &BezierSeed) -> Vec<f64> {
    // Mimic bezier-js `extrema().values`, then MakerJs.getExtrema's rounding + uniqueness.
    let pts = seed.points();
    let order = pts.len() - 1;

    // derive points: utils.derive
    let mut dpoints: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut p = pts;
    let mut d = p.len();
    let mut c = d - 1;
    while d > 1 {
        let mut list: Vec<[f64; 2]> = Vec::new();
        for j in 0..c {
            list.push([
                c as f64 * (p[j + 1][0] - p[j][0]),
                c as f64 * (p[j + 1][1] - p[j][1]),
            ]);
        }
        dpoints.push(list.clone());
        p = list;
        d = p.len();
        c = d - 1;
    }

    let mut roots: Vec<f64> = Vec::new();
    for dim in 0..2 {
        let mut r: Vec<f64> = Vec::new();
        let p0: Vec<f64> = dpoints[0].iter().map(|v| v[dim]).collect();
        r.extend(droots(&p0));
        if order == 3 {
            let p1: Vec<f64> = dpoints[1].iter().map(|v| v[dim]).collect();
            r.extend(droots(&p1));
        }
        r.retain(|t| *t >= 0.0 && *t <= 1.0);
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        roots.extend(r);
    }

    roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    roots.dedup_by(|a, b| a == b);

    // MakerJs.getExtrema rounds these t values and dedups again.
    let mut extrema: Vec<f64> = roots.into_iter().map(|t| makerjs_round(t, 1e-7)).collect();
    extrema.sort_by(|a, b| a.partial_cmp(b).unwrap());
    extrema.dedup();

    if extrema.is_empty() {
        return vec![0.0, 1.0];
    }
    if extrema[0] != 0.0 {
        extrema.insert(0, 0.0);
    }
    if *extrema.last().unwrap() != 1.0 {
        extrema.push(1.0);
    }
    extrema
}

fn bezier_error(seed: &BezierSeed, start_t: f64, end_t: f64, arc: MakerArc, arc_reversed: bool) -> f64 {
    let t_span = end_t - start_t;
    let m = |ratio: f64| {
        let t = start_t + t_span * ratio;
        let bp = bezier_compute(seed, t);
        let ap = arc_point_at_ratio(arc, if arc_reversed { 1.0 - ratio } else { ratio });
        point_distance(ap, bp)
    };
    m(0.25) + m(0.75)
}

fn path_length(seg: &ArcOrLine) -> f64 {
    match seg {
        ArcOrLine::Line { a, b, .. } => point_distance(*a, *b),
        ArcOrLine::Arc { arc, .. } => arc.radius * arc_span_deg(*arc) * PI / 180.0,
    }
}

fn get_largest_arc(seed: &BezierSeed, start_t: f64, end_t: f64, accuracy: f64) -> ArcOrLine {
    let start = TPoint {
        t: start_t,
        point: bezier_compute(seed, start_t),
    };
    let end = TPoint {
        t: end_t,
        point: bezier_compute(seed, end_t),
    };
    let mut upper = end;
    let mut lower = start;
    let mut count = 0;
    let mut test = upper;
    let mut last_good: Option<MakerArc> = None;
    let mut reversed: Option<bool> = None;

    while count < 100 {
        let middle = bezier_compute(seed, (start.t + test.t) / 2.0);
        let arc = match arc_from_3_points(start.point, middle, test.point) {
            Ok(a) => a,
            Err(_) => {
                if let Some(a) = last_good {
                    return ArcOrLine::Arc { arc: a, end_t: lower.t };
                }
                break;
            }
        };

        if reversed.is_none() {
            let arc_end_point = arc_point(arc, arc.end_angle_deg);
            reversed = Some(points_equal_xy_round(start.point, arc_end_point));
        }

        let arc_reversed = reversed.unwrap_or(false);
        let err = bezier_error(seed, start_t, test.t, arc, arc_reversed);
        let accepted = err <= accuracy;
        if accepted {
            last_good = Some(arc);
            lower = test;
        } else {
            upper = test;
        }

        if lower.t == upper.t {
            if let Some(a) = last_good {
                return ArcOrLine::Arc { arc: a, end_t: lower.t };
            }
        }

        if !accepted
            && let Some(good) = last_good
            && (arc_span_deg(arc) - arc_span_deg(good)) < 0.5
        {
            return ArcOrLine::Arc {
                arc: good,
                end_t: lower.t,
            };
        }

        count += 1;
        test = TPoint {
            t: (lower.t + upper.t) / 2.0,
            point: bezier_compute(seed, (lower.t + upper.t) / 2.0),
        };
    }

    ArcOrLine::Line {
        a: start.point,
        b: test.point,
        end_t: test.t,
    }
}

pub fn bezier_curve_primitives(
    seed: BezierSeed,
    accuracy: Option<f64>,
) -> Vec<Primitive> {
    // MakerJs BezierCurve:
    // - if linear, output a single line.
    // - otherwise approximate via arcs/lines.

    // Linear check (very small subset; sufficient for our fixtures).
    if seed.controls.iter().all(|c| {
        let cross = (seed.end[0] - seed.origin[0]) * (c[1] - seed.origin[1])
            - (seed.end[1] - seed.origin[1]) * (c[0] - seed.origin[0]);
        makerjs_round(cross, 1e-7) == 0.0
    }) {
        return vec![Primitive::Line {
            a: seed.origin,
            b: seed.end,
        }];
    }

    let accuracy = accuracy.unwrap_or_else(|| bezier_length(&seed) / 100.0);
    let extrema = bezier_extrema_values(&seed);

    let mut out: Vec<Primitive> = Vec::new();
    for i in 1..extrema.len() {
        let span = extrema[i] - extrema[i - 1];
        let acc = accuracy * span;
        let mut start_t = extrema[i - 1];
        let end_t = extrema[i];
        while start_t < end_t {
            let seg = get_largest_arc(&seed, start_t, end_t, acc);
            start_t = match &seg {
                ArcOrLine::Arc { end_t, .. } => *end_t,
                ArcOrLine::Line { end_t, .. } => *end_t,
            };
            if path_length(&seg) < 0.0001 {
                continue;
            }
            match seg {
                ArcOrLine::Arc { arc, .. } => {
                    let (a, b) = arc_endpoints(arc);
                    out.push(Primitive::Arc {
                        arc,
                        a,
                        b,
                        reversed: false,
                    });
                }
                ArcOrLine::Line { a, b, .. } => out.push(Primitive::Line { a, b }),
            }
        }
    }

    out
}

fn angle_mirror(angle_deg: f64, mirror_x: bool, mirror_y: bool) -> f64 {
    // MakerJs.angle.mirror
    let mut a = angle_deg;
    if mirror_y {
        a = 360.0 - a;
    }
    if mirror_x {
        a = (if a < 180.0 { 180.0 } else { 540.0 }) - a;
    }
    a
}

fn mirror_arc(arc: MakerArc, mirror_x: bool, mirror_y: bool) -> MakerArc {
    // MakerJs.path.mirrorMap[Arc]
    let origin = [
        if mirror_x { -arc.origin[0] } else { arc.origin[0] },
        if mirror_y { -arc.origin[1] } else { arc.origin[1] },
    ];
    let start = angle_mirror(arc.start_angle_deg, mirror_x, mirror_y);
    let end = angle_mirror(arc_end_angle_deg(arc), mirror_x, mirror_y);
    let xor = mirror_x != mirror_y;
    MakerArc {
        origin,
        radius: arc.radius,
        start_angle_deg: if xor { end } else { start },
        end_angle_deg: if xor { start } else { end },
    }
}

pub fn s_curve_primitives(from: [f64; 2], to: [f64; 2]) -> Result<Vec<Primitive>, MakerJsPathError> {
    // Port of MakerJs.models.SCurve plus the `outlines.js` mirror+move logic.
    if from[0] == to[0] {
        return Err(MakerJsPathError::NotClosedChain);
    }
    if from[1] == to[1] {
        return Err(MakerJsPathError::NotClosedChain);
    }
    let width = (to[0] - from[0]).abs();
    let height = (to[1] - from[1]).abs();
    let mirror_x = from[0] > to[0];
    let mirror_y = from[1] > to[1];

    let find_radius = |x: f64, y: f64| x + (y * y - x * x) / (2.0 * x);
    let h2 = height / 2.0;
    let w2 = width / 2.0;

    let (radius, start_angle, end_angle, arc_origin) = if width > height {
        let r = find_radius(h2, w2);
        let end = 360.0 - (w2 / r).acos().to_degrees();
        (r, 270.0, end, [0.0, r])
    } else {
        let r = find_radius(w2, h2);
        let start = 180.0 - (h2 / r).asin().to_degrees();
        (r, start, 180.0, [r, 0.0])
    };

    let curve_start = MakerArc {
        origin: arc_origin,
        radius,
        start_angle_deg: start_angle,
        end_angle_deg: end_angle,
    };

    // curve_end = moveRelative(mirror(curve_start, true, true), [width, height])
    let curve_end = {
        let mut a = mirror_arc(curve_start, true, true);
        a.origin[0] += width;
        a.origin[1] += height;
        a
    };

    // model mirror + move (outlines.js)
    let mut arcs = vec![curve_start, curve_end];
    if mirror_x || mirror_y {
        arcs = arcs.into_iter().map(|a| mirror_arc(a, mirror_x, mirror_y)).collect();
    }
    for a in &mut arcs {
        a.origin[0] += from[0];
        a.origin[1] += from[1];
    }

    Ok(arcs
        .into_iter()
        .map(|arc| {
            let a = arc_point(arc, arc.start_angle_deg);
            let b = arc_point(arc, arc.end_angle_deg);
            Primitive::Arc {
                arc,
                a,
                b,
                reversed: false,
            }
        })
        .collect())
}

pub fn chain_to_closed_polyline(
    segments: Vec<Primitive>,
    start_hint: Option<[f64; 2]>,
) -> Result<Polyline<f64>, MakerJsPathError> {
    if segments.is_empty() {
        return Err(MakerJsPathError::NotClosedChain);
    }

    let mut segs = segments;
    let start = start_hint.unwrap_or_else(|| segs[0].endpoints().0);
    let mut current = start;

    // MakerJS has a configurable `pointMatchingDistance` for chaining; our fixture comparer later
    // quantizes at 1e-6, so use that as our chaining epsilon too.
    const CHAIN_EPS: f64 = 1e-6;

    let mut pl = Polyline::new_closed();
    for (segment_index, seg) in segs.drain(..).enumerate() {
        let (a, b) = seg.endpoints();
        let seg = if points_equal_xy_within(a, current, CHAIN_EPS) {
            seg
        } else if points_equal_xy_within(b, current, CHAIN_EPS) {
            seg.reversed()
        } else {
            return Err(MakerJsPathError::Disconnected {
                at: current,
                segment_index,
                segment_a: a,
                segment_b: b,
            });
        };

        let (a, b) = seg.endpoints();
        debug_assert!(points_equal_xy_within(a, current, CHAIN_EPS));

        let bulge = match seg {
            Primitive::Line { .. } => 0.0,
            Primitive::Arc { arc, reversed, .. } => {
                let span = arc_span_deg(arc) * PI / 180.0;
                let mut bulge = (span / 4.0).tan();
                if reversed {
                    bulge = -bulge;
                }
                bulge
            }
        };

        pl.vertex_data.push(PlineVertex::new(a[0], a[1], bulge));
        current = b;
    }

    if !points_equal_xy_within(current, start, CHAIN_EPS) {
        return Err(MakerJsPathError::NotClosedChain);
    }

    // Ensure closed; remove duplicate last vertex if present (cavalier treats closed by flag).
    if pl.vertex_data.len() >= 2 {
        let first = pl.vertex_data[0];
        let last = *pl.vertex_data.last().unwrap();
        if points_equal_xy_within([first.x, first.y], [last.x, last.y], CHAIN_EPS) {
            pl.vertex_data.pop();
        }
    }

    // If we accidentally duplicated arc starts, de-dup consecutive identical vertices.
    let mut cleaned: Vec<PlineVertex<f64>> = Vec::with_capacity(pl.vertex_data.len());
    for v in pl.vertex_data.iter().copied() {
        if cleaned
            .last()
            .is_some_and(|p| points_equal_xy_within([p.x, p.y], [v.x, v.y], CHAIN_EPS))
        {
            continue;
        }
        cleaned.push(v);
    }
    pl.vertex_data = cleaned;
    pl.is_closed = true;

    Ok(pl)
}

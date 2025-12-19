//! A small, dependency-free port of the `hull` JS library used by upstream Ergogen.
//!
//! Important: Ergogen depends on a `hull` fork which embeds its own `convex.js` and `intersect.js`
//! helpers (it does **not** use `monotone-convex-hull-2d` / `robust-segment-intersect`).
//! This module ports the exact algorithms from:
//! - `/tmp/ergogen-upstream/node_modules/hull/src/hull.js`
//! - `/tmp/ergogen-upstream/node_modules/hull/src/convex.js`
//! - `/tmp/ergogen-upstream/node_modules/hull/src/intersect.js`

use std::collections::{HashMap, HashSet};

const MAX_SEARCH_BBOX_SIZE_PERCENT: f64 = 0.6;
// `Math.cos(Math.PI / 2)` in JS.
const MAX_CONCAVE_ANGLE_COS: f64 = 6.123_233_995_736_766e-17_f64;

#[derive(Debug, Clone, Copy)]
struct Pt {
    x: f64,
    y: f64,
}

impl Pt {
    fn from_xy(p: [f64; 2]) -> Self {
        Self { x: p[0], y: p[1] }
    }

    fn to_xy(self) -> [f64; 2] {
        [self.x, self.y]
    }
}

fn sort_by_x(mut pts: Vec<Pt>) -> Vec<Pt> {
    pts.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap().then_with(|| a.y.partial_cmp(&b.y).unwrap()));
    pts
}

fn filter_duplicates(sorted: Vec<Pt>) -> Vec<Pt> {
    if sorted.is_empty() {
        return sorted;
    }
    let mut unique = Vec::with_capacity(sorted.len());
    unique.push(sorted[0]);
    let mut last = sorted[0];
    for p in sorted.into_iter().skip(1) {
        if last.x != p.x || last.y != p.y {
            unique.push(p);
        }
        last = p;
    }
    unique
}

fn sq_length(a: Pt, b: Pt) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dx * dx + dy * dy
}

fn cos_at(o: Pt, a: Pt, b: Pt) -> f64 {
    let a_shifted = Pt {
        x: a.x - o.x,
        y: a.y - o.y,
    };
    let b_shifted = Pt {
        x: b.x - o.x,
        y: b.y - o.y,
    };
    let sq_a = sq_length(o, a);
    let sq_b = sq_length(o, b);
    let dot = a_shifted.x * b_shifted.x + a_shifted.y * b_shifted.y;
    dot / (sq_a * sq_b).sqrt()
}

fn occupied_area(points: &[Pt]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    (max_x - min_x, max_y - min_y)
}

fn bbox_around(edge: [Pt; 2]) -> [f64; 4] {
    [
        edge[0].x.min(edge[1].x), // left
        edge[0].y.min(edge[1].y), // top
        edge[0].x.max(edge[1].x), // right
        edge[0].y.max(edge[1].y), // bottom
    ]
}

// --- convex.js --------------------------------------------------------------

fn ccw_pop(p1: Pt, p2: Pt, p3: Pt) -> bool {
    // convex.js: cross <= 0
    (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x) <= 0.0
}

fn tangent(pointset: &[Pt]) -> Vec<Pt> {
    let mut res: Vec<Pt> = Vec::new();
    for &p in pointset {
        while res.len() > 1 {
            let n = res.len();
            if ccw_pop(res[n - 2], res[n - 1], p) {
                res.pop();
            } else {
                break;
            }
        }
        res.push(p);
    }
    res.pop();
    res
}

fn convex_in_place(pointset: &mut Vec<Pt>) -> Vec<Pt> {
    // pointset must be sorted by X.
    let upper = tangent(pointset);
    // NOTE: convex.js reverses the input array in-place and leaves it reversed. `hull.js` relies on
    // this side effect when computing `innerPoints` via `points.filter(...)`.
    pointset.reverse();
    let lower = tangent(pointset);
    let mut convex = lower;
    convex.extend(upper);
    convex.push(pointset[0]); // close with pointset[0] after reverse (matches convex.js)
    convex
}

// --- intersect.js -----------------------------------------------------------

fn ccw_bool(a: Pt, b: Pt, c: Pt) -> bool {
    // intersect.js:
    //   cw = ((y3 - y1) * (x2 - x1)) - ((y2 - y1) * (x3 - x1))
    //   return cw > 0 ? true : cw < 0 ? false : true; // colinear
    let cw = (c.y - a.y) * (b.x - a.x) - (b.y - a.y) * (c.x - a.x);
    if cw > 0.0 {
        true
    } else if cw < 0.0 {
        false
    } else {
        true
    }
}

fn segments_intersect(seg1: [Pt; 2], seg2: [Pt; 2]) -> bool {
    // intersect.js: ccw(p1,p3,p4) !== ccw(p2,p3,p4) && ccw(p1,p2,p3) !== ccw(p1,p2,p4)
    ccw_bool(seg1[0], seg2[0], seg2[1]) != ccw_bool(seg1[1], seg2[0], seg2[1])
        && ccw_bool(seg1[0], seg1[1], seg2[0]) != ccw_bool(seg1[0], seg1[1], seg2[1])
}

fn intersects(segment: [Pt; 2], pointset: &[Pt]) -> bool {
    for i in 0..pointset.len().saturating_sub(1) {
        let seg = [pointset[i], pointset[i + 1]];
        if (segment[0].x == seg[0].x && segment[0].y == seg[0].y)
            || (segment[0].x == seg[1].x && segment[0].y == seg[1].y)
        {
            continue;
        }
        if segments_intersect(segment, seg) {
            return true;
        }
    }
    false
}

// --- grid.js ---------------------------------------------------------------

#[derive(Debug)]
struct Grid {
    cells: HashMap<(i32, i32), Vec<Pt>>,
    cell_size: f64,
    reverse_cell_size: f64,
}

impl Grid {
    fn new(points: &[Pt], cell_size: f64) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<Pt>> = HashMap::new();
        let reverse_cell_size = 1.0 / cell_size;

        for &p in points {
            let cx = Self::coord_to_cell_num(p.x, reverse_cell_size);
            let cy = Self::coord_to_cell_num(p.y, reverse_cell_size);
            cells.entry((cx, cy)).or_default().push(p);
        }

        Self {
            cells,
            cell_size,
            reverse_cell_size,
        }
    }

    fn coord_to_cell_num(x: f64, reverse_cell_size: f64) -> i32 {
        // grid.js uses `Math.trunc(x * reverseCellSize)`
        (x * reverse_cell_size).trunc() as i32
    }

    fn cell_points(&self, x: i32, y: i32) -> &[Pt] {
        self.cells.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[])
    }

    fn range_points(&self, bbox: [f64; 4]) -> Vec<Pt> {
        let tl_x = Self::coord_to_cell_num(bbox[0], self.reverse_cell_size);
        let tl_y = Self::coord_to_cell_num(bbox[1], self.reverse_cell_size);
        let br_x = Self::coord_to_cell_num(bbox[2], self.reverse_cell_size);
        let br_y = Self::coord_to_cell_num(bbox[3], self.reverse_cell_size);

        let mut out = Vec::new();
        for x in tl_x..=br_x {
            for y in tl_y..=br_y {
                out.extend_from_slice(self.cell_points(x, y));
            }
        }
        out
    }

    fn remove_point(&mut self, p: Pt) {
        let cx = Self::coord_to_cell_num(p.x, self.reverse_cell_size);
        let cy = Self::coord_to_cell_num(p.y, self.reverse_cell_size);
        let Some(cell) = self.cells.get_mut(&(cx, cy)) else {
            return;
        };
        if let Some(idx) = cell
            .iter()
            .position(|q| q.x == p.x && q.y == p.y)
        {
            cell.remove(idx);
        }
    }

    fn extend_bbox(&self, bbox: [f64; 4], scale_factor: i32) -> [f64; 4] {
        [
            bbox[0] - (scale_factor as f64 * self.cell_size),
            bbox[1] - (scale_factor as f64 * self.cell_size),
            bbox[2] + (scale_factor as f64 * self.cell_size),
            bbox[3] + (scale_factor as f64 * self.cell_size),
        ]
    }
}

// --- hull.js ---------------------------------------------------------------

fn midpoint_candidate(edge: [Pt; 2], inner_points: &[Pt], convex: &[Pt]) -> Option<Pt> {
    let mut point: Option<Pt> = None;
    let mut angle1_cos = MAX_CONCAVE_ANGLE_COS;
    let mut angle2_cos = MAX_CONCAVE_ANGLE_COS;

    for &p in inner_points {
        let a1 = cos_at(edge[0], edge[1], p);
        let a2 = cos_at(edge[1], edge[0], p);

        if a1 > angle1_cos
            && a2 > angle2_cos
            && !intersects([edge[0], p], convex)
            && !intersects([edge[1], p], convex)
        {
            angle1_cos = a1;
            angle2_cos = a2;
            point = Some(p);
        }
    }
    point
}

fn concave(
    convex: &mut Vec<Pt>,
    max_sq_edge_len: f64,
    max_search_area: (f64, f64),
    grid: &mut Grid,
    edge_skip_list: &mut HashSet<String>,
) {
    let mut inserted = false;

    // JS uses a `for` loop while mutating `convex`. `convex.length` is re-evaluated each
    // iteration, so we use a `while` loop here to match that dynamic behavior.
    let mut i = 0usize;
    while i + 1 < convex.len() {
        let edge = [convex[i], convex[i + 1]];
        let key = format!("{},{},{},{}", edge[0].x, edge[0].y, edge[1].x, edge[1].y);

        if sq_length(edge[0], edge[1]) < max_sq_edge_len || edge_skip_list.contains(&key) {
            i += 1;
            continue;
        }

        let mut scale_factor = 0i32;
        let mut bbox = bbox_around(edge);
        let mut bbox_w: f64;
        let mut bbox_h: f64;
        let mut mid: Option<Pt>;

        loop {
            bbox = grid.extend_bbox(bbox, scale_factor);
            bbox_w = bbox[2] - bbox[0];
            bbox_h = bbox[3] - bbox[1];

            mid = midpoint_candidate(edge, &grid.range_points(bbox), convex);
            scale_factor += 1;

            if mid.is_some() {
                break;
            }
            if !(max_search_area.0 > bbox_w || max_search_area.1 > bbox_h) {
                break;
            }
        }

        if bbox_w >= max_search_area.0 && bbox_h >= max_search_area.1 {
            edge_skip_list.insert(key);
        }

        if let Some(p) = mid {
            convex.insert(i + 1, p);
            grid.remove_point(p);
            inserted = true;
        }

        i += 1;
    }

    if inserted {
        concave(convex, max_sq_edge_len, max_search_area, grid, edge_skip_list);
    }
}

/// Compute a concave hull / polygon around the given `pointset` using the upstream `hull` module,
/// returning a closed point list (first point repeated at the end).
pub fn hull(pointset: Vec<[f64; 2]>, concavity: f64) -> Vec<[f64; 2]> {
    let max_edge_len = if concavity == 0.0 { 20.0 } else { concavity };

    let points = sort_by_x(pointset.into_iter().map(Pt::from_xy).collect());
    let mut points = filter_duplicates(points);

    if points.len() < 4 {
        let mut out = points.into_iter().map(|p| p.to_xy()).collect::<Vec<_>>();
        if let Some(first) = out.first().copied() {
            out.push(first);
        }
        return out;
    }

    let (occ_w, occ_h) = occupied_area(&points);
    let max_search_area = (
        occ_w * MAX_SEARCH_BBOX_SIZE_PERCENT,
        occ_h * MAX_SEARCH_BBOX_SIZE_PERCENT,
    );

    let mut convex = convex_in_place(&mut points);

    let mut inner_points: Vec<Pt> = Vec::new();
    for p in &points {
        if !convex.iter().any(|c| c.x == p.x && c.y == p.y) {
            inner_points.push(*p);
        }
    }

    let denom = occ_w * occ_h;
    let cell_size = (1.0 / (points.len() as f64 / denom)).ceil();

    let mut grid = Grid::new(&inner_points, cell_size);
    let mut skip = HashSet::<String>::new();
    concave(
        &mut convex,
        max_edge_len * max_edge_len,
        max_search_area,
        &mut grid,
        &mut skip,
    );

    convex.into_iter().map(|p| p.to_xy()).collect()
}

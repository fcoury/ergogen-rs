use cavalier_contours::polyline::{
    BooleanOp, BooleanResultInfo, PlineOrientation, PlineSource, PlineSourceMut, Polyline,
};

#[derive(Debug, Clone, Default)]
pub struct Region {
    pub pos: Vec<Polyline<f64>>,
    pub neg: Vec<Polyline<f64>>,
}

impl Region {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_pos(pos: Vec<Polyline<f64>>) -> Self {
        Self { pos, neg: vec![] }
    }

    pub fn union_all(mut plines: Vec<Polyline<f64>>) -> Self {
        // Drop degenerate polylines early.
        plines.retain(|p| p.is_closed() && p.vertex_count() >= 2);
        plines = plines.into_iter().map(simplify).collect();
        let (pos, neg) = union_pline_set_with_holes(plines);
        Self {
            pos: normalize_winding(pos, PlineOrientation::CounterClockwise),
            neg: normalize_winding(neg, PlineOrientation::Clockwise),
        }
    }

    pub fn subtract_all(&mut self, cutters: &[Polyline<f64>]) {
        let mut new_pos: Vec<Polyline<f64>> = Vec::new();
        let mut new_neg: Vec<Polyline<f64>> = Vec::new();

        for p in std::mem::take(&mut self.pos) {
            let mut cur_pos = vec![p];
            let mut cur_neg: Vec<Polyline<f64>> = Vec::new();

            for c in cutters {
                let mut next_pos: Vec<Polyline<f64>> = Vec::new();
                let mut next_neg: Vec<Polyline<f64>> = Vec::new();

                for cp in cur_pos {
                    let res = cp.boolean(c, BooleanOp::Not);
                    next_pos.extend(res.pos_plines.into_iter().map(|p| p.pline));
                    next_neg.extend(res.neg_plines.into_iter().map(|p| p.pline));
                }

                cur_pos = next_pos;
                cur_neg.extend(next_neg);
            }

            new_pos.extend(cur_pos);
            new_neg.extend(cur_neg);
        }

        self.pos = normalize_winding(union_pline_set(new_pos), PlineOrientation::CounterClockwise);
        self.neg
            .extend(normalize_winding(union_pline_set(new_neg), PlineOrientation::Clockwise));
    }
}

fn union_pline_set(mut plines: Vec<Polyline<f64>>) -> Vec<Polyline<f64>> {
    // O(n^2) pairwise merge until stable; fixture sizes are tiny.
    plines.retain(|p| p.is_closed() && p.vertex_count() >= 2);
    plines = plines.into_iter().map(simplify).collect();

    let mut i = 0usize;
    while i < plines.len() {
        let mut merged = false;
        let mut j = i + 1;
        while j < plines.len() {
            let res = plines[i].boolean(&plines[j], BooleanOp::Or);
            match res.result_info {
                BooleanResultInfo::Disjoint => {
                    j += 1;
                    continue;
                }
                BooleanResultInfo::InvalidInput => {
                    j += 1;
                    continue;
                }
                _ => {
                    let mut next: Vec<Polyline<f64>> = res
                        .pos_plines
                        .into_iter()
                        .map(|p| simplify(p.pline))
                        .collect();
                    // Replace plines[i] and plines[j] with the union results.
                    plines.swap_remove(j);
                    plines.swap_remove(i);
                    plines.append(&mut next);
                    merged = true;
                    break;
                }
            }
        }
        if merged {
            i = 0;
        } else {
            i += 1;
        }
    }

    plines
}

fn union_pline_set_with_holes(
    mut plines: Vec<Polyline<f64>>,
) -> (Vec<Polyline<f64>>, Vec<Polyline<f64>>) {
    // Same pairwise-union strategy as `union_pline_set`, but preserve any hole polylines produced
    // by CavalierContours (these are needed for some upstream outline fixtures).
    plines.retain(|p| p.is_closed() && p.vertex_count() >= 2);
    plines = plines.into_iter().map(simplify).collect();

    let mut holes: Vec<Polyline<f64>> = Vec::new();

    let mut i = 0usize;
    while i < plines.len() {
        let mut merged = false;
        let mut j = i + 1;
        while j < plines.len() {
            let res = plines[i].boolean(&plines[j], BooleanOp::Or);
            match res.result_info {
                BooleanResultInfo::Disjoint => {
                    j += 1;
                    continue;
                }
                BooleanResultInfo::InvalidInput => {
                    j += 1;
                    continue;
                }
                _ => {
                    let mut next: Vec<Polyline<f64>> = res
                        .pos_plines
                        .into_iter()
                        .map(|p| simplify(p.pline))
                        .collect();
                    holes.extend(res.neg_plines.into_iter().map(|p| simplify(p.pline)));

                    // Replace plines[i] and plines[j] with the union results.
                    plines.swap_remove(j);
                    plines.swap_remove(i);
                    plines.append(&mut next);
                    merged = true;
                    break;
                }
            }
        }
        if merged {
            i = 0;
        } else {
            i += 1;
        }
    }

    holes = union_pline_set(holes);
    (plines, holes)
}

fn normalize_winding(
    plines: Vec<Polyline<f64>>,
    desired: PlineOrientation,
) -> Vec<Polyline<f64>> {
    plines
        .into_iter()
        .map(|mut pl| {
            let orientation = pl.orientation();
            if orientation != PlineOrientation::Open && orientation != desired {
                pl.invert_direction_mut();
            }
            pl
        })
        .collect()
}

fn simplify(p: Polyline<f64>) -> Polyline<f64> {
    // Upstream outputs often collapse collinear segments; match that by removing redundant vertexes
    // after boolean operations.
    p.remove_redundant(1e-6).unwrap_or(p)
}

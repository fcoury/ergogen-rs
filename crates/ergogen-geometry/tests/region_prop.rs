use cavalier_contours::polyline::{PlineOrientation, PlineSource, PlineSourceMut};
use ergogen_geometry::primitives::{rectangle, rounded_rectangle};
use ergogen_geometry::region::Region;
use ergogen_geometry::Polyline;
use proptest::prelude::*;

fn bbox(pl: &Polyline<f64>) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for v in &pl.vertex_data {
        min_x = min_x.min(v.x);
        min_y = min_y.min(v.y);
        max_x = max_x.max(v.x);
        max_y = max_y.max(v.y);
    }
    (min_x, min_y, max_x, max_y)
}

fn assert_bbox_close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let eps = 1e-6;
    (a.0 - b.0).abs() < eps
        && (a.1 - b.1).abs() < eps
        && (a.2 - b.2).abs() < eps
        && (a.3 - b.3).abs() < eps
}

fn region_is_simple(region: &Region) -> bool {
    region
        .pos
        .iter()
        .chain(region.neg.iter())
        .all(|pl| pl.is_closed() && !pl.scan_for_self_intersect())
}

fn region_has_valid_winding(region: &Region) -> bool {
    region
        .pos
        .iter()
        .chain(region.neg.iter())
        .all(|pl| pl.orientation() != PlineOrientation::Open)
}

fn region_has_expected_winding(region: &Region) -> bool {
    region
        .pos
        .iter()
        .all(|pl| pl.orientation() == PlineOrientation::CounterClockwise)
        && region
            .neg
            .iter()
            .all(|pl| pl.orientation() == PlineOrientation::Clockwise)
}

fn sorted_bboxes(region: &Region) -> Vec<(i64, i64, i64, i64)> {
    let eps = 1e-6;
    let quant = |v: f64| (v / eps).round() as i64;
    let mut out: Vec<_> = region
        .pos
        .iter()
        .chain(region.neg.iter())
        .map(|pl| {
            let (min_x, min_y, max_x, max_y) = bbox(pl);
            (quant(min_x), quant(min_y), quant(max_x), quant(max_y))
        })
        .collect();
    out.sort();
    out
}

proptest! {
    #[test]
    fn union_all_preserves_disjoint_rectangles(
        w1 in 1.0f64..50.0,
        h1 in 1.0f64..50.0,
        w2 in 1.0f64..50.0,
        h2 in 1.0f64..50.0,
        gap in 5.0f64..50.0,
    ) {
        let r1 = rectangle((0.0, 0.0), (w1, h1), 0.0);
        let offset_x = (w1 / 2.0) + (w2 / 2.0) + gap;
        let r2 = rectangle((offset_x, 0.0), (w2, h2), 0.0);

        let region = Region::union_all(vec![r1.clone(), r2.clone()]);
        prop_assert!(region.neg.is_empty());
        prop_assert_eq!(region.pos.len(), 2);
        prop_assert!(region_is_simple(&region));
        prop_assert!(region_has_valid_winding(&region));
        prop_assert!(region_has_expected_winding(&region));

        let mut got: Vec<_> = region.pos.iter().map(bbox).collect();
        let mut expected = vec![bbox(&r1), bbox(&r2)];
        got.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        expected.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (g, e) in got.into_iter().zip(expected.into_iter()) {
            prop_assert!(assert_bbox_close(g, e));
        }
    }

    #[test]
    fn union_all_overlapping_rectangles_are_simple(
        w1 in 2.0f64..50.0,
        h1 in 2.0f64..50.0,
        w2 in 2.0f64..50.0,
        h2 in 2.0f64..50.0,
        overlap in 0.25f64..10.0,
    ) {
        prop_assume!(overlap < w1.min(w2) / 2.0);
        let r1 = rectangle((0.0, 0.0), (w1, h1), 0.0);
        let offset_x = (w1 / 2.0) + (w2 / 2.0) - overlap;
        let r2 = rectangle((offset_x, 0.0), (w2, h2), 0.0);

        let region = Region::union_all(vec![r1, r2]);
        prop_assert!(region_is_simple(&region));
        prop_assert!(region_has_valid_winding(&region));
        prop_assert!(region_has_expected_winding(&region));
    }

    #[test]
    fn subtracting_identical_rectangle_clears_region(
        w in 1.0f64..50.0,
        h in 1.0f64..50.0,
    ) {
        let rect = rectangle((0.0, 0.0), (w, h), 0.0);
        let mut region = Region::from_pos(vec![rect.clone()]);
        region.subtract_all(&[rect]);
        prop_assert!(region.pos.is_empty());
        prop_assert!(region.neg.is_empty());
    }

    #[test]
    fn subtracting_inner_rectangle_produces_simple_hole(
        w in 5.0f64..50.0,
        h in 5.0f64..50.0,
        inset in 0.5f64..10.0,
    ) {
        prop_assume!(inset < w / 2.0);
        prop_assume!(inset < h / 2.0);
        let outer = rectangle((0.0, 0.0), (w, h), 0.0);
        let inner = rectangle((0.0, 0.0), (w - inset * 2.0, h - inset * 2.0), 0.0);
        let mut region = Region::from_pos(vec![outer]);
        region.subtract_all(&[inner]);
        prop_assert!(region_is_simple(&region));
        prop_assert!(region_has_valid_winding(&region));
        prop_assert!(region_has_expected_winding(&region));
    }

    #[test]
    fn union_all_order_invariant_for_disjoint_rectangles(
        w1 in 2.0f64..30.0,
        h1 in 2.0f64..30.0,
        w2 in 2.0f64..30.0,
        h2 in 2.0f64..30.0,
        w3 in 2.0f64..30.0,
        h3 in 2.0f64..30.0,
        gap in 5.0f64..20.0,
    ) {
        let r1 = rectangle((0.0, 0.0), (w1, h1), 0.0);
        let r2 = rectangle(((w1 / 2.0) + (w2 / 2.0) + gap, 0.0), (w2, h2), 0.0);
        let r3 = rectangle((0.0, (h1 / 2.0) + (h3 / 2.0) + gap), (w3, h3), 0.0);

        let region_a = Region::union_all(vec![r1.clone(), r2.clone(), r3.clone()]);
        let region_b = Region::union_all(vec![r3, r2, r1]);

        prop_assert!(region_is_simple(&region_a));
        prop_assert!(region_is_simple(&region_b));
        prop_assert_eq!(sorted_bboxes(&region_a), sorted_bboxes(&region_b));
        prop_assert!(region_has_expected_winding(&region_a));
        prop_assert!(region_has_expected_winding(&region_b));
    }

    #[test]
    fn rounded_rectangle_is_simple(
        w in 5.0f64..50.0,
        h in 5.0f64..50.0,
        r in 0.5f64..10.0,
    ) {
        prop_assume!(r < w / 2.0);
        prop_assume!(r < h / 2.0);
        let pl = rounded_rectangle((0.0, 0.0), (w, h), r, 0.0);
        let region = Region::union_all(vec![pl]);
        prop_assert!(region_is_simple(&region));
        prop_assert!(region_has_valid_winding(&region));
        prop_assert!(region_has_expected_winding(&region));
    }
}

#[test]
fn union_all_normalizes_clockwise_input() {
    let mut rect = rectangle((0.0, 0.0), (10.0, 8.0), 0.0);
    rect.invert_direction_mut();
    let region = Region::union_all(vec![rect]);
    assert!(region_has_expected_winding(&region));
}

#[test]
fn subtract_all_normalizes_hole_winding() {
    let outer = rectangle((0.0, 0.0), (10.0, 10.0), 0.0);
    let mut inner = rectangle((0.0, 0.0), (6.0, 6.0), 0.0);
    inner.invert_direction_mut();
    let mut region = Region::from_pos(vec![outer]);
    region.subtract_all(&[inner]);
    assert!(!region.neg.is_empty());
    assert!(region_has_expected_winding(&region));
}

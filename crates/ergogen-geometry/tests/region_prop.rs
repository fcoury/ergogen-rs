use ergogen_geometry::primitives::rectangle;
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

        let mut got: Vec<_> = region.pos.iter().map(bbox).collect();
        let mut expected = vec![bbox(&r1), bbox(&r2)];
        got.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        expected.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (g, e) in got.into_iter().zip(expected.into_iter()) {
            prop_assert!(assert_bbox_close(g, e));
        }
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
}

use indexmap::IndexMap;

use ergogen_core::{Point, PointMeta};
use ergogen_layout::anchor::parse_anchor;
use ergogen_parser::{Units, Value};

fn units() -> Units {
    Units::parse(None, None).unwrap()
}

fn v(j: serde_json::Value) -> Value {
    Value::try_from_json_str(&j.to_string()).unwrap()
}

fn assert_point(p: &Point, x: f64, y: f64, r: f64) {
    let eps = 1e-9;
    assert!((p.x - x).abs() <= eps, "x got={} expected={x}", p.x);
    assert!((p.y - y).abs() <= eps, "y got={} expected={y}", p.y);
    assert!((p.r - r).abs() <= eps, "r got={} expected={r}", p.r);
}

fn sample_points() -> IndexMap<String, Point> {
    IndexMap::from([
        (
            "o".to_string(),
            Point::new(0.0, 0.0, 0.0, PointMeta::default()),
        ),
        (
            "rotated_o".to_string(),
            Point::new(0.0, 0.0, 90.0, PointMeta::default()),
        ),
        (
            "o_five".to_string(),
            Point::new(0.0, 5.0, 0.0, PointMeta::default()),
        ),
        (
            "five_o".to_string(),
            Point::new(5.0, 0.0, 0.0, PointMeta::default()),
        ),
        (
            "five".to_string(),
            Point::new(5.0, 5.0, 90.0, PointMeta::default()),
        ),
        (
            "ten".to_string(),
            Point::new(10.0, 10.0, -90.0, PointMeta::default()),
        ),
        (
            "mirror_ten".to_string(),
            Point::new(-10.0, 10.0, 90.0, PointMeta { mirrored: true }),
        ),
    ])
}

#[test]
fn anchor_params_and_ref() {
    let units = units();
    let points = sample_points();

    assert_point(
        &parse_anchor(
            &v(serde_json::json!({})),
            "name",
            &points,
            Point::xy(0.0, 0.0),
            &units,
            false,
        )
        .unwrap(),
        0.0,
        0.0,
        0.0,
    );

    assert_point(
        &parse_anchor(
            &v(serde_json::json!({"ref":"o"})),
            "name",
            &points,
            Point::xy(0.0, 0.0),
            &units,
            false,
        )
        .unwrap(),
        0.0,
        0.0,
        0.0,
    );

    assert_point(
        &parse_anchor(
            &v(serde_json::json!({})),
            "name",
            &points,
            Point::xy(1.0, 1.0),
            &units,
            false,
        )
        .unwrap(),
        1.0,
        1.0,
        0.0,
    );

    assert_point(
        &parse_anchor(
            &v(serde_json::json!({"ref":"ten"})),
            "name",
            &points,
            Point::xy(0.0, 0.0),
            &units,
            true,
        )
        .unwrap(),
        -10.0,
        10.0,
        90.0,
    );
}

#[test]
fn anchor_recursive_ref() {
    let units = units();
    let points = sample_points();
    let raw = v(serde_json::json!({
        "ref": { "ref": "o", "shift": [2, 2] }
    }));
    let p = parse_anchor(&raw, "name", &points, Point::xy(0.0, 0.0), &units, false).unwrap();
    assert_point(&p, 2.0, 2.0, 0.0);
}

#[test]
fn anchor_aggregate_average() {
    let units = units();
    let points = sample_points();
    let raw = v(serde_json::json!({
        "aggregate": { "parts": ["o", "ten"] }
    }));
    let p = parse_anchor(&raw, "name", &points, Point::xy(0.0, 0.0), &units, false).unwrap();
    assert_point(&p, 5.0, 5.0, -45.0);
}

#[test]
fn anchor_aggregate_intersect() {
    let units = units();
    let points = sample_points();

    let raw = v(serde_json::json!({
        "aggregate": { "parts": ["o", "ten"], "method": "intersect" }
    }));
    let p = parse_anchor(&raw, "name", &points, Point::xy(0.0, 0.0), &units, false).unwrap();
    assert_point(&p, 0.0, 10.0, 0.0);

    let raw = v(serde_json::json!({
        "aggregate": { "parts": ["o", "five_o"], "method": "intersect" }
    }));
    let err = parse_anchor(&raw, "name", &points, Point::xy(0.0, 0.0), &units, false).unwrap_err();
    assert!(err.to_string().contains("do not intersect"));
}

#[test]
fn anchor_orient_rotate_and_affect() {
    let units = units();
    let points = sample_points();

    let p = parse_anchor(
        &v(serde_json::json!({"orient": -90, "shift": [0, 1]})),
        "name",
        &points,
        Point::xy(0.0, 0.0),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 1.0, 0.0, -90.0);

    let p = parse_anchor(
        &v(serde_json::json!({"orient": "ten", "shift": [0, std::f64::consts::SQRT_2]})),
        "name",
        &points,
        Point::xy(0.0, 0.0),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 1.0, 1.0, -45.0);

    let p = parse_anchor(
        &v(serde_json::json!({"shift": [0, 1], "rotate": -90})),
        "name",
        &points,
        Point::xy(0.0, 0.0),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 0.0, 1.0, -90.0);

    let p = parse_anchor(
        &v(serde_json::json!({"rotate": {"shift": [-1, -1]}})),
        "name",
        &points,
        Point::xy(0.0, 0.0),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 0.0, 0.0, 135.0);

    let p = parse_anchor(
        &v(serde_json::json!({"orient": -90, "shift": [0, 1], "rotate": 10, "affect": "r"})),
        "name",
        &points,
        Point::xy(0.0, 0.0),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 0.0, 0.0, -80.0);
}

#[test]
fn anchor_resist_on_mirrored_points() {
    let units = units();
    let points = sample_points();

    let base = Point::new(0.0, 0.0, 0.0, PointMeta { mirrored: true });
    let p = parse_anchor(
        &v(serde_json::json!({"shift": [1, 1]})),
        "name",
        &points,
        base.clone(),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, -1.0, 1.0, 0.0);

    let p = parse_anchor(
        &v(serde_json::json!({"shift": [1, 1], "resist": true})),
        "name",
        &points,
        base.clone(),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 1.0, 1.0, 0.0);

    let p = parse_anchor(
        &v(serde_json::json!({"rotate": 10})),
        "name",
        &points,
        base.clone(),
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 0.0, 0.0, -10.0);

    let p = parse_anchor(
        &v(serde_json::json!({"rotate": 10, "resist": true})),
        "name",
        &points,
        base,
        &units,
        false,
    )
    .unwrap();
    assert_point(&p, 0.0, 0.0, 10.0);
}

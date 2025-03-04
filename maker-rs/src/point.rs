/// Rotate a point.
///
/// # Arguments
///
/// * `point` - The point to rotate.
/// * `angle` - The amount of rotation, in degrees.
/// * `origin` - The center point of rotation, defaults to (0.0, 0.0).
///
/// # Returns
///
/// A new point after rotation.
pub fn rotate(point: (f64, f64), angle: f64, origin: Option<(f64, f64)>) -> (f64, f64) {
    let origin = origin.unwrap_or((0.0, 0.0));

    // Calculate the angle of the point relative to the origin in radians
    let point_angle = (point.1 - origin.1).atan2(point.0 - origin.0);

    // Calculate the distance between the origin and the point
    let distance = ((point.0 - origin.0).powi(2) + (point.1 - origin.1).powi(2)).sqrt();

    // Convert the angle to radians and add it to the point's angle
    let new_angle = point_angle + angle.to_radians();

    // Convert from polar coordinates back to Cartesian
    let rotated_point = (new_angle.cos() * distance, new_angle.sin() * distance);

    // Add the rotated point to the origin to get the final position
    (origin.0 + rotated_point.0, origin.1 + rotated_point.1)
}

use std::path::Path;

use ergogen_export::dxf::{Arc, Circle, Dxf, DxfError, Entity, Line, LwPolyline, Point2};
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Debug, thiserror::Error)]
pub enum Dxf2PngError {
    #[error("DXF error: {0}")]
    Dxf(#[from] DxfError),
    #[error("failed to create pixmap with dimensions {width}x{height}")]
    PixmapCreation { width: u32, height: u32 },
    #[error("PNG encoding error: {0}")]
    PngEncode(#[from] image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Width of the output image in pixels.
    pub width: u32,
    /// Height of the output image in pixels.
    pub height: u32,
    /// Padding around the drawing in pixels.
    pub padding: u32,
    /// Stroke width for lines and curves.
    pub stroke_width: f32,
    /// Background color as RGBA.
    pub background: [u8; 4],
    /// Stroke color as RGBA.
    pub stroke_color: [u8; 4],
    /// Number of segments to approximate arcs.
    pub arc_segments: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            padding: 20,
            stroke_width: 2.0,
            background: [255, 255, 255, 255], // white
            stroke_color: [0, 0, 0, 255],     // black
            arc_segments: 64,
        }
    }
}

/// Converts a DXF file to PNG format.
pub fn dxf_to_png(
    dxf_path: impl AsRef<Path>,
    opts: &RenderOptions,
) -> Result<Vec<u8>, Dxf2PngError> {
    let dxf = Dxf::parse_file(dxf_path)?;
    render_dxf_to_png(&dxf, opts)
}

/// Converts a DXF string to PNG format.
pub fn dxf_str_to_png(dxf_content: &str, opts: &RenderOptions) -> Result<Vec<u8>, Dxf2PngError> {
    let dxf = Dxf::parse_str(dxf_content)?;
    render_dxf_to_png(&dxf, opts)
}

/// Renders a parsed DXF to PNG bytes.
pub fn render_dxf_to_png(dxf: &Dxf, opts: &RenderOptions) -> Result<Vec<u8>, Dxf2PngError> {
    let pixmap = render_dxf(dxf, opts)?;
    encode_png(&pixmap)
}

/// Renders a DXF to a Pixmap.
pub fn render_dxf(dxf: &Dxf, opts: &RenderOptions) -> Result<Pixmap, Dxf2PngError> {
    let mut pixmap = Pixmap::new(opts.width, opts.height).ok_or(Dxf2PngError::PixmapCreation {
        width: opts.width,
        height: opts.height,
    })?;

    // Fill background
    let bg = tiny_skia::Color::from_rgba8(
        opts.background[0],
        opts.background[1],
        opts.background[2],
        opts.background[3],
    );
    pixmap.fill(bg);

    // Calculate bounding box
    let bbox = calculate_bbox(&dxf.entities);
    if bbox.is_none() {
        return Ok(pixmap); // Empty DXF
    }
    let (min_x, min_y, max_x, max_y) = bbox.unwrap();

    // Calculate scale and offset to fit drawing in viewport
    let drawing_width = max_x - min_x;
    let drawing_height = max_y - min_y;

    let available_width = (opts.width - 2 * opts.padding) as f64;
    let available_height = (opts.height - 2 * opts.padding) as f64;

    let scale = if drawing_width == 0.0 && drawing_height == 0.0 {
        1.0
    } else if drawing_width == 0.0 {
        available_height / drawing_height
    } else if drawing_height == 0.0 {
        available_width / drawing_width
    } else {
        (available_width / drawing_width).min(available_height / drawing_height)
    };

    let scaled_width = drawing_width * scale;
    let scaled_height = drawing_height * scale;

    let offset_x = opts.padding as f64 + (available_width - scaled_width) / 2.0;
    let offset_y = opts.padding as f64 + (available_height - scaled_height) / 2.0;

    // Create paint and stroke
    let mut paint = Paint::default();
    paint.set_color(tiny_skia::Color::from_rgba8(
        opts.stroke_color[0],
        opts.stroke_color[1],
        opts.stroke_color[2],
        opts.stroke_color[3],
    ));
    paint.anti_alias = true;

    let stroke = Stroke {
        width: opts.stroke_width,
        ..Default::default()
    };

    // Transform function: DXF coords -> pixel coords
    // Note: Y is flipped (DXF Y+ is up, image Y+ is down)
    let transform_point = |p: Point2| -> (f32, f32) {
        let x = ((p.x - min_x) * scale + offset_x) as f32;
        let y = ((max_y - p.y) * scale + offset_y) as f32;
        (x, y)
    };

    // Render each entity
    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => {
                render_line(&mut pixmap, line, &transform_point, &paint, &stroke);
            }
            Entity::Circle(circle) => {
                render_circle(
                    &mut pixmap,
                    circle,
                    &transform_point,
                    scale,
                    &paint,
                    &stroke,
                );
            }
            Entity::Arc(arc) => {
                render_arc(
                    &mut pixmap,
                    arc,
                    &transform_point,
                    scale,
                    opts.arc_segments,
                    &paint,
                    &stroke,
                );
            }
            Entity::LwPolyline(polyline) => {
                render_polyline(
                    &mut pixmap,
                    polyline,
                    &transform_point,
                    scale,
                    opts.arc_segments,
                    &paint,
                    &stroke,
                );
            }
            Entity::Unsupported(_) => {
                // Skip unsupported entities
            }
        }
    }

    Ok(pixmap)
}

fn calculate_bbox(entities: &[Entity]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut has_points = false;

    for entity in entities {
        match entity {
            Entity::Line(line) => {
                has_points = true;
                min_x = min_x.min(line.start.x).min(line.end.x);
                min_y = min_y.min(line.start.y).min(line.end.y);
                max_x = max_x.max(line.start.x).max(line.end.x);
                max_y = max_y.max(line.start.y).max(line.end.y);
            }
            Entity::Circle(circle) => {
                has_points = true;
                min_x = min_x.min(circle.center.x - circle.radius);
                min_y = min_y.min(circle.center.y - circle.radius);
                max_x = max_x.max(circle.center.x + circle.radius);
                max_y = max_y.max(circle.center.y + circle.radius);
            }
            Entity::Arc(arc) => {
                // For simplicity, use the full circle bbox
                has_points = true;
                min_x = min_x.min(arc.center.x - arc.radius);
                min_y = min_y.min(arc.center.y - arc.radius);
                max_x = max_x.max(arc.center.x + arc.radius);
                max_y = max_y.max(arc.center.y + arc.radius);
            }
            Entity::LwPolyline(polyline) => {
                for v in &polyline.vertices {
                    has_points = true;
                    min_x = min_x.min(v.x);
                    min_y = min_y.min(v.y);
                    max_x = max_x.max(v.x);
                    max_y = max_y.max(v.y);
                }
            }
            Entity::Unsupported(_) => {}
        }
    }

    if has_points {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn render_line<F>(pixmap: &mut Pixmap, line: &Line, transform: &F, paint: &Paint, stroke: &Stroke)
where
    F: Fn(Point2) -> (f32, f32),
{
    let (x1, y1) = transform(line.start);
    let (x2, y2) = transform(line.end);

    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);

    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, paint, stroke, Transform::identity(), None);
    }
}

fn render_circle<F>(
    pixmap: &mut Pixmap,
    circle: &Circle,
    transform: &F,
    scale: f64,
    paint: &Paint,
    stroke: &Stroke,
) where
    F: Fn(Point2) -> (f32, f32),
{
    let (cx, cy) = transform(circle.center);
    let r = (circle.radius * scale) as f32;

    if let Some(path) = circle_path(cx, cy, r) {
        pixmap.stroke_path(&path, paint, stroke, Transform::identity(), None);
    }
}

fn circle_path(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Path> {
    // Approximate circle with cubic beziers (4 segments)
    let k = 0.552_284_8; // (4/3) * tan(pi/8)
    let c = r * k;

    let mut pb = PathBuilder::new();
    pb.move_to(cx + r, cy);
    pb.cubic_to(cx + r, cy + c, cx + c, cy + r, cx, cy + r);
    pb.cubic_to(cx - c, cy + r, cx - r, cy + c, cx - r, cy);
    pb.cubic_to(cx - r, cy - c, cx - c, cy - r, cx, cy - r);
    pb.cubic_to(cx + c, cy - r, cx + r, cy - c, cx + r, cy);
    pb.close();
    pb.finish()
}

fn render_arc<F>(
    pixmap: &mut Pixmap,
    arc: &Arc,
    transform: &F,
    _scale: f64,
    segments: usize,
    paint: &Paint,
    stroke: &Stroke,
) where
    F: Fn(Point2) -> (f32, f32),
{
    let points = arc_to_points(arc, segments);
    if points.is_empty() {
        return;
    }

    let mut pb = PathBuilder::new();
    let (x0, y0) = transform(points[0]);
    pb.move_to(x0, y0);

    for &p in &points[1..] {
        let (x, y) = transform(p);
        pb.line_to(x, y);
    }

    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, paint, stroke, Transform::identity(), None);
    }
}

fn arc_to_points(arc: &Arc, segments: usize) -> Vec<Point2> {
    let mut start_deg = arc.start_angle_deg;
    let mut end_deg = arc.end_angle_deg;

    // Normalize angles
    while start_deg < 0.0 {
        start_deg += 360.0;
    }
    while end_deg < 0.0 {
        end_deg += 360.0;
    }
    while start_deg >= 360.0 {
        start_deg -= 360.0;
    }
    while end_deg >= 360.0 {
        end_deg -= 360.0;
    }

    // DXF arcs go counter-clockwise
    let mut sweep = end_deg - start_deg;
    if sweep <= 0.0 {
        sweep += 360.0;
    }

    let num_points = ((segments as f64 * sweep / 360.0).ceil() as usize).max(2);
    let mut points = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        let angle_deg = start_deg + t * sweep;
        let angle_rad = angle_deg.to_radians();
        let x = arc.center.x + arc.radius * angle_rad.cos();
        let y = arc.center.y + arc.radius * angle_rad.sin();
        points.push(Point2 { x, y });
    }

    points
}

fn render_polyline<F>(
    pixmap: &mut Pixmap,
    polyline: &LwPolyline,
    transform: &F,
    _scale: f64,
    arc_segments: usize,
    paint: &Paint,
    stroke: &Stroke,
) where
    F: Fn(Point2) -> (f32, f32),
{
    if polyline.vertices.is_empty() {
        return;
    }

    let mut pb = PathBuilder::new();
    let (x0, y0) = transform(polyline.vertices[0]);
    pb.move_to(x0, y0);

    let n = polyline.vertices.len();
    let vertex_count = if polyline.closed { n } else { n - 1 };

    for i in 0..vertex_count {
        let start = polyline.vertices[i];
        let end = polyline.vertices[(i + 1) % n];
        let bulge = polyline.bulges.get(i).copied().unwrap_or(0.0);

        if bulge.abs() < 1e-9 {
            // Straight line segment
            let (x, y) = transform(end);
            pb.line_to(x, y);
        } else {
            // Arc segment
            let arc_points = bulge_arc_points(start, end, bulge, arc_segments);
            for &p in &arc_points[1..] {
                let (x, y) = transform(p);
                pb.line_to(x, y);
            }
        }
    }

    if polyline.closed {
        pb.close();
    }

    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, paint, stroke, Transform::identity(), None);
    }
}

fn bulge_arc_points(start: Point2, end: Point2, bulge: f64, segments: usize) -> Vec<Point2> {
    // Bulge = tan(arc_angle / 4)
    let theta = 4.0 * bulge.atan();
    let chord_len = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();

    if chord_len < 1e-9 {
        return vec![start, end];
    }

    let radius = chord_len / (2.0 * (theta / 2.0).sin().abs());
    let sagitta = bulge * chord_len / 2.0;

    // Midpoint of chord
    let mid_x = (start.x + end.x) / 2.0;
    let mid_y = (start.y + end.y) / 2.0;

    // Perpendicular direction
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let perp_x = -dy / chord_len;
    let perp_y = dx / chord_len;

    // Distance from midpoint to center
    let h = radius - sagitta.abs();
    let sign = if bulge > 0.0 { 1.0 } else { -1.0 };

    let center_x = mid_x + sign * h * perp_x;
    let center_y = mid_y + sign * h * perp_y;

    // Angles
    let start_angle = (start.y - center_y).atan2(start.x - center_x);
    let end_angle = (end.y - center_y).atan2(end.x - center_x);

    let num_points =
        ((segments as f64 * theta.abs() / (2.0 * std::f64::consts::PI)).ceil() as usize).max(2);

    let mut points = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        let angle = if bulge > 0.0 {
            // Counter-clockwise
            let mut sweep = end_angle - start_angle;
            if sweep < 0.0 {
                sweep += 2.0 * std::f64::consts::PI;
            }
            start_angle + t * sweep
        } else {
            // Clockwise
            let mut sweep = start_angle - end_angle;
            if sweep < 0.0 {
                sweep += 2.0 * std::f64::consts::PI;
            }
            start_angle - t * sweep
        };

        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        points.push(Point2 { x, y });
    }

    points
}

fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, Dxf2PngError> {
    use image::{ImageBuffer, Rgba};

    let width = pixmap.width();
    let height = pixmap.height();
    let data = pixmap.data();

    // tiny-skia uses premultiplied alpha, convert to straight alpha
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let idx = (y * width + x) as usize * 4;
        let r = data[idx];
        let g = data[idx + 1];
        let b = data[idx + 2];
        let a = data[idx + 3];

        // Unpremultiply alpha
        let (r, g, b) = if a == 0 {
            (0, 0, 0)
        } else if a == 255 {
            (r, g, b)
        } else {
            let af = a as f32 / 255.0;
            (
                (r as f32 / af).min(255.0) as u8,
                (g as f32 / af).min(255.0) as u8,
                (b as f32 / af).min(255.0) as u8,
            )
        };

        *pixel = Rgba([r, g, b, a]);
    }

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(buf)
}

/// Save a DXF file as PNG to the given path.
pub fn save_dxf_as_png(
    dxf_path: impl AsRef<Path>,
    png_path: impl AsRef<Path>,
    opts: &RenderOptions,
) -> Result<(), Dxf2PngError> {
    let png_data = dxf_to_png(dxf_path, opts)?;
    std::fs::write(png_path, png_data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dxf() {
        let dxf_content = r#"0
SECTION
2
ENTITIES
0
ENDSEC
0
EOF
"#;
        let result = dxf_str_to_png(dxf_content, &RenderOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_line() {
        let dxf_content = r#"0
SECTION
2
ENTITIES
0
LINE
10
0
20
0
11
100
21
100
0
ENDSEC
0
EOF
"#;
        let result = dxf_str_to_png(dxf_content, &RenderOptions::default());
        assert!(result.is_ok());
        let png_data = result.unwrap();
        assert!(!png_data.is_empty());
        // Check PNG signature
        assert_eq!(
            &png_data[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn test_circle() {
        let dxf_content = r#"0
SECTION
2
ENTITIES
0
CIRCLE
10
50
20
50
40
25
0
ENDSEC
0
EOF
"#;
        let result = dxf_str_to_png(dxf_content, &RenderOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_arc() {
        let dxf_content = r#"0
SECTION
2
ENTITIES
0
ARC
10
50
20
50
40
25
50
0
51
90
0
ENDSEC
0
EOF
"#;
        let result = dxf_str_to_png(dxf_content, &RenderOptions::default());
        assert!(result.is_ok());
    }
}

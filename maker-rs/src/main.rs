// fn main() {
//     let mut model = Model::new();
//     model.add_path(
//         "line1",
//         Path::Line(Line::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0))),
//     );
//     model.add_path(
//         "circle1",
//         Path::Circle(Circle::new(Point::new(0.0, 0.0), 50.0)),
//     );
//
//     let svg_options = SvgRenderOptions {
//         stroke: Some("black".to_string()),
//         stroke_width: Some("1".to_string()),
//         fill: Some("none".to_string()),
//         view_box: true,
//         ..Default::default()
//     };
//
//     let svg = to_svg(&model, &svg_options);
//     println!("{}", svg);
// }
//
// #[derive(Debug, Clone, Copy)]
// pub struct Point {
//     x: f64,
//     y: f64,
// }
//
// impl Point {
//     pub fn new(x: f64, y: f64) -> Self {
//         Self { x, y }
//     }
//
//     // Mirror point along Y axis for SVG conversion
//     pub fn to_svg_coords(&self) -> Self {
//         Self {
//             x: self.x,
//             y: -self.y,
//         }
//     }
// }
//
// #[derive(Debug, Clone)]
// pub enum Path {
//     Line(Line),
//     Circle(Circle),
//     Arc(Arc),
// }
//
// impl Path {
//     pub fn get_extents(&self) -> (Point, Point) {
//         match self {
//             Path::Line(line) => line.get_extents(),
//             Path::Circle(circle) => circle.get_extents(),
//             Path::Arc(arc) => arc.get_extents(),
//         }
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct Line {
//     origin: Point,
//     end: Point,
// }
//
// impl Line {
//     pub fn new(origin: Point, end: Point) -> Self {
//         Self { origin, end }
//     }
//
//     pub fn get_extents(&self) -> (Point, Point) {
//         let low_x = self.origin.x.min(self.end.x);
//         let low_y = self.origin.y.min(self.end.y);
//         let high_x = self.origin.x.max(self.end.x);
//         let high_y = self.origin.y.max(self.end.y);
//
//         (Point::new(low_x, low_y), Point::new(high_x, high_y))
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct Circle {
//     origin: Point,
//     radius: f64,
// }
//
// impl Circle {
//     pub fn new(origin: Point, radius: f64) -> Self {
//         Self { origin, radius }
//     }
//
//     pub fn get_extents(&self) -> (Point, Point) {
//         let low = Point::new(self.origin.x - self.radius, self.origin.y - self.radius);
//         let high = Point::new(self.origin.x + self.radius, self.origin.y + self.radius);
//         (low, high)
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct Arc {
//     origin: Point,
//     radius: f64,
//     start_angle: f64,
//     end_angle: f64,
// }
//
// impl Arc {
//     pub fn new(origin: Point, radius: f64, start_angle: f64, end_angle: f64) -> Self {
//         Self {
//             origin,
//             radius,
//             start_angle,
//             end_angle,
//         }
//     }
//
//     pub fn get_extents(&self) -> (Point, Point) {
//         // TODO: improve arc extents
//         let low = Point::new(self.origin.x - self.radius, self.origin.y - self.radius);
//         let high = Point::new(self.origin.x + self.radius, self.origin.y + self.radius);
//         (low, high)
//     }
// }
//
// // Model structure to hold paths
// #[derive(Debug, Default)]
// pub struct Model {
//     paths: std::collections::HashMap<String, Path>,
// }
//
// impl Model {
//     pub fn new() -> Self {
//         Self {
//             paths: std::collections::HashMap::new(),
//         }
//     }
//
//     pub fn add_path(&mut self, id: &str, path: Path) {
//         self.paths.insert(id.to_string(), path);
//     }
//
//     pub fn get_extents(&self) -> Option<(Point, Point)> {
//         if self.paths.is_empty() {
//             return None;
//         }
//
//         let mut min_x = f64::MAX;
//         let mut min_y = f64::MAX;
//         let mut max_x = f64::MIN;
//         let mut max_y = f64::MIN;
//
//         for path in self.paths.values() {
//             let (low, high) = path.get_extents();
//             min_x = min_x.min(low.x);
//             min_y = min_y.min(low.y);
//             max_x = max_x.max(high.x);
//             max_y = max_y.max(high.y);
//         }
//
//         Some((Point::new(min_x, min_y), Point::new(max_x, max_y)))
//     }
// }
//
// // SVG rendering options
// #[derive(Debug, Clone)]
// pub struct SvgRenderOptions {
//     pub stroke: Option<String>,
//     pub stroke_width: Option<String>,
//     pub fill: Option<String>,
//     pub view_box: bool,
//     pub accuracy: usize,
//     pub origin: Option<Point>,
//     pub scale: f64,
// }
//
// impl Default for SvgRenderOptions {
//     fn default() -> Self {
//         Self {
//             stroke: None,
//             stroke_width: None,
//             fill: None,
//             view_box: true,
//             accuracy: 3,
//             origin: None,
//             scale: 1.0,
//         }
//     }
// }
//
// fn round(value: f64, accuracy: usize) -> f64 {
//     let factor = 10.0_f64.powi(accuracy as i32);
//     (value * factor).round() / factor
// }
//
// pub trait ToSvg {
//     fn to_svg(&self, options: &SvgRenderOptions, offset: Point) -> String;
// }
//
// impl ToSvg for Path {
//     fn to_svg(&self, options: &SvgRenderOptions, offset: Point) -> String {
//         match self {
//             Path::Line(line) => line.to_svg(options, offset),
//             Path::Circle(circle) => circle.to_svg(options, offset),
//             Path::Arc(arc) => arc.to_svg(options, offset),
//         }
//     }
// }
//
// impl ToSvg for Line {
//     fn to_svg(&self, options: &SvgRenderOptions, offset: Point) -> String {
//         // Apply coordinate transformation and offset
//         let start = self.origin.to_svg_coords();
//         let end = self.end.to_svg_coords();
//
//         let x1 = round((start.x + offset.x) * options.scale, options.accuracy);
//         let y1 = round((start.y + offset.y) * options.scale, options.accuracy);
//         let x2 = round((end.x + offset.x) * options.scale, options.accuracy);
//         let y2 = round((end.y + offset.y) * options.scale, options.accuracy);
//
//         let mut attrs = format!("x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"", x1, y1, x2, y2);
//
//         if let Some(stroke) = &options.stroke {
//             attrs.push_str(&format!(" stroke=\"{}\"", stroke));
//         }
//
//         if let Some(stroke_width) = &options.stroke_width {
//             attrs.push_str(&format!(" stroke-width=\"{}\"", stroke_width));
//         }
//
//         format!("<line {} />", attrs)
//     }
// }
//
// impl ToSvg for Circle {
//     fn to_svg(&self, options: &SvgRenderOptions, offset: Point) -> String {
//         // Apply coordinate transformation and offset
//         let center = self.origin.to_svg_coords();
//
//         let cx = round((center.x + offset.x) * options.scale, options.accuracy);
//         let cy = round((center.y + offset.y) * options.scale, options.accuracy);
//         let r = round(self.radius * options.scale, options.accuracy);
//
//         let mut attrs = format!("cx=\"{}\" cy=\"{}\" r=\"{}\"", cx, cy, r);
//
//         if let Some(stroke) = &options.stroke {
//             attrs.push_str(&format!(" stroke=\"{}\"", stroke));
//         }
//
//         if let Some(stroke_width) = &options.stroke_width {
//             attrs.push_str(&format!(" stroke-width=\"{}\"", stroke_width));
//         }
//
//         if let Some(fill) = &options.fill {
//             attrs.push_str(&format!(" fill=\"{}\"", fill));
//         }
//
//         format!("<circle {} />", attrs)
//     }
// }
//
// impl ToSvg for Arc {
//     fn to_svg(&self, options: &SvgRenderOptions, offset: Point) -> String {
//         // This is a simplified version - a full implementation would handle proper arc conversion
//         // to SVG path data with proper flags for sweep and large arc
//
//         let center = self.origin.to_svg_coords();
//         let cx = round((center.x + offset.x) * options.scale, options.accuracy);
//         let cy = round((center.y + offset.y) * options.scale, options.accuracy);
//         let r = round(self.radius * options.scale, options.accuracy);
//
//         // Calculate start and end points
//         let start_x = cx + r * (self.start_angle * std::f64::consts::PI / 180.0).cos();
//         let start_y = cy + r * (self.start_angle * std::f64::consts::PI / 180.0).sin();
//         let end_x = cx + r * (self.end_angle * std::f64::consts::PI / 180.0).cos();
//         let end_y = cy + r * (self.end_angle * std::f64::consts::PI / 180.0).sin();
//
//         // Determine if the arc is large (> 180 degrees)
//         let large_arc = (self.end_angle - self.start_angle).abs() > 180.0;
//
//         // Path data for the arc
//         let path_data = format!(
//             "M {} {} A {} {} 0 {} 1 {} {}",
//             round(start_x, options.accuracy),
//             round(start_y, options.accuracy),
//             r,
//             r, // x and y radii
//             if large_arc { 1 } else { 0 },
//             round(end_x, options.accuracy),
//             round(end_y, options.accuracy)
//         );
//
//         let mut attrs = format!("d=\"{}\"", path_data);
//
//         if let Some(stroke) = &options.stroke {
//             attrs.push_str(&format!(" stroke=\"{}\"", stroke));
//         }
//
//         if let Some(stroke_width) = &options.stroke_width {
//             attrs.push_str(&format!(" stroke-width=\"{}\"", stroke_width));
//         }
//
//         if let Some(fill) = &options.fill {
//             attrs.push_str(&format!(" fill=\"{}\"", fill));
//         }
//
//         format!("<path {} />", attrs)
//     }
// }
//
// pub fn to_svg(model: &Model, options: &SvgRenderOptions) -> String {
//     let extents = model.get_extents();
//
//     let (width, height, view_box, offset) = if let Some((low, high)) = extents {
//         let stroke_width_val = options
//             .stroke_width
//             .as_ref()
//             .map_or(0.0, |s| s.parse().unwrap());
//         let padding = stroke_width_val / 2.0; // Add padding for stroke width
//
//         let width = (high.x - low.x) * options.scale + (padding * 2.0);
//         let height = (high.y - low.y) * options.scale + (padding * 2.0);
//
//         let view_box = format!("{} {} {} {}", -padding, padding, width, height);
//
//         // Calculate proper offset for the SVG coordinate system
//         let offset = if let Some(origin) = options.origin {
//             origin
//         } else {
//             Point::new(-low.x, high.y) // Place at top-left corner, flipping Y
//         };
//
//         (width, height, view_box, offset)
//     } else {
//         (
//             100.0,
//             100.0,
//             "0 0 100 100".to_string(),
//             Point::new(0.0, 0.0),
//         )
//     };
//
//     let mut svg = String::new();
//     svg.push_str(&format!(
//         "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"{}>\n",
//         width,
//         height,
//         if options.view_box {
//             format!(" viewBox=\"{}\"", view_box)
//         } else {
//             "".to_string()
//         }
//     ));
//
//     // Add a group element to contain all paths
//     svg.push_str(
//         "<g stroke-linecap=\"round\" fill=\"none\" vector-effect=\"non-scaling-stroke\">\n",
//     );
//
//     for path in model.paths.values() {
//         svg.push_str(&format!("  {}\n", path.to_svg(options, offset)));
//     }
//
//     svg.push_str("</g>\n</svg>");
//     svg
// }

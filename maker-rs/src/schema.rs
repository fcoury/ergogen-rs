use indexmap::IndexMap;

use crate::paths::{Path, PathLine};

pub type Point = (f64, f64);

pub enum PathType {
    Path,
    Arc,
    Circle,
    Line,
}

/// Text annotation, diplayable natively to the output format.
///
pub struct Caption {
    /// Caption text.
    pub text: String,
    /// Invisible line to which the text is aligned.
    /// The text will be horizontally and vertically centered on the center point of this line.
    /// The text may be longer or shorter than the line, it is used only for position and angle.
    /// The anchor line's endpoints may be omitted, in which case the text will always remain
    /// non-angled, even if the model is rotated.
    pub anchor: Box<dyn PathLine>,
}

/// A model is a composite object which may contain a map of paths, or a map of models recursively.
///
/// Example:
/// ```
/// let m = Model {
///     paths: Some(indexmap! {
///         "line1".to_string() => /* path object */,
///         "line2".to_string() => /* path object */
///     }),
///     ..Default::default()
/// };
/// ```
pub struct Model {
    /// A model may want to specify its type, but this value is not employed yet.
    pub typ_: Option<String>,
    /// Optional origin location of this model.
    pub origin: Point,
    /// Optional map of path objects in this model.
    pub paths: Option<IndexMap<String, Box<dyn Path>>>,
    /// Optional map of models within this model.
    pub models: Option<IndexMap<String, Model>>,
    /// Optional unit system of this model. See UnitType for possible values.
    pub units: Option<String>,
    /// An author may wish to add notes to this model instance.
    pub notes: Option<String>,
    /// Optional layer of this model.
    pub layer: Option<String>,
    /// Optional Caption object.
    pub caption: Option<Caption>,
    // TODO: /// Optional exporter options for this model. -- not used by ergogen
    // pub exporter_options: Option<IndexMap<String, serde_json::Value>>,
}

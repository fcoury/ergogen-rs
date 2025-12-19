use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to parse JSON: {0}")]
    Json(String),

    #[error("invalid expression for \"{key}\": {expr}")]
    InvalidExpression { key: String, expr: String },

    #[error("unknown variable \"{name}\" referenced while evaluating \"{key}\"")]
    UnknownVariable { key: String, name: String },

    #[error("expression evaluation failed for \"{key}\": {message}")]
    Eval { key: String, message: String },

    #[error("YAML mapping keys must be strings")]
    NonStringKey,

    #[error("unsupported YAML value (tags/anchors are not supported in IR yet)")]
    UnsupportedYamlValue,

    #[error("YAML number could not be represented as f64")]
    YamlNumber,

    #[error("invalid dotted path \"{path}\" at segment \"{segment}\"")]
    InvalidPath { path: String, segment: String },

    #[error("inheritance target \"{path}\" does not exist (reached from \"{from}\")")]
    ExtendsTargetMissing { from: String, path: String },

    #[error("circular $extends dependency detected: {cycle}")]
    ExtendsCycle { cycle: String },

    #[error("parameterization error at \"{at}\": {message}")]
    Parameterize { at: String, message: String },

    #[error("\"units\" and \"variables\" must be YAML mappings")]
    UnitsNotMap,

    #[error("unit/variable \"{key}\" must be a number or string expression")]
    UnitsValueType { key: String },
}

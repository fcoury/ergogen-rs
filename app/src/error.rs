use crate::expr;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("YAML parsing error at {location}: {message}")]
    YamlParse {
        message: String,
        location: String,

        #[source]
        source: serde_yaml::Error,
    },

    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Expression parsing error: {0}")]
    ExprParse(#[from] expr::ParserError),

    #[error("Invalid config: {0}")]
    Config(String),

    #[error("Input format error: {0}")]
    Format(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Type error: expected {expected} for field {field}")]
    TypeError { field: String, expected: String },

    #[error("Value error: {0}")]
    ValueError(String),

    #[error("{message} in anchor \"{name}\"")]
    AnchorParse { message: String, name: String },

    #[error("Expected a number value in \"{0}\", got \"{1}\"")]
    UnitParse(String, String),
}

impl From<serde_yaml::Error> for Error {
    fn from(error: serde_yaml::Error) -> Self {
        println!("{:?}", error);
        let location = format!(
            "line {}, column {}",
            error.location().map_or(0, |loc| loc.line()),
            error.location().map_or(0, |loc| loc.column())
        );

        Error::YamlParse {
            message: error.to_string(),
            location,
            source: error,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarExpr {
    Number(f64),
    String(String),
}

impl ScalarExpr {
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ScalarExpr::String(s) => Some(s),
            ScalarExpr::Number(_) => None,
        }
    }
}

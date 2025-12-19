use indexmap::IndexMap;
use serde::Deserialize;

use crate::expr::ScalarExpr;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawConfig {
    #[serde(default)]
    pub units: Option<IndexMap<String, ScalarExpr>>,

    #[serde(default)]
    pub variables: Option<IndexMap<String, ScalarExpr>>,
}

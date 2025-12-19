//! Parsing, preprocessing, and expression evaluation.

mod config;
mod error;
mod eval;
mod expr;
mod prepare;
mod units;
mod value;

pub use config::RawConfig;
pub use error::Error;
pub use eval::eval_in_context;
pub use expr::ScalarExpr;
pub use prepare::{PreparedIr, extend_all, inherit, parameterize, unnest};
pub use units::{UnitEntry, Units};
pub use value::Value;

#[derive(Debug, Clone)]
pub struct PreparedConfig {
    /// Canonical (preprocessed) configuration: unnest → inherit → parameterize.
    pub canonical: Value,
    pub units: Units,
}

impl PreparedConfig {
    pub fn from_yaml_str(yaml: &str) -> Result<Self, Error> {
        let ir = PreparedIr::from_yaml_str(yaml)?;
        let units = units_from_canonical(&ir.canonical)?;
        Ok(Self {
            canonical: ir.canonical,
            units,
        })
    }
}

fn units_from_canonical(canonical: &Value) -> Result<Units, Error> {
    let units = canonical.get_path("units");
    let variables = canonical.get_path("variables");

    let units_map = units
        .map(value_map_to_scalar_expr_map)
        .transpose()?
        .unwrap_or_default();
    let vars_map = variables
        .map(value_map_to_scalar_expr_map)
        .transpose()?
        .unwrap_or_default();

    let units_map_opt = if units_map.is_empty() {
        None
    } else {
        Some(&units_map)
    };
    let vars_map_opt = if vars_map.is_empty() {
        None
    } else {
        Some(&vars_map)
    };
    Units::parse(units_map_opt, vars_map_opt)
}

fn value_map_to_scalar_expr_map(
    v: &Value,
) -> Result<indexmap::IndexMap<String, ScalarExpr>, Error> {
    let Some(map) = v.as_map() else {
        return Err(Error::UnitsNotMap);
    };
    let mut out = indexmap::IndexMap::new();
    for (k, v) in map {
        let expr = match v {
            Value::Number(n) => ScalarExpr::Number(*n),
            Value::String(s) => ScalarExpr::String(s.clone()),
            _ => return Err(Error::UnitsValueType { key: k.clone() }),
        };
        out.insert(k.clone(), expr);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_yaml_produces_predictable_units_snapshot() {
        let yaml = r#"
units:
  u: 20
variables:
  foo: u + 2
"#;

        let prepared = PreparedConfig::from_yaml_str(yaml).unwrap();
        let snap = prepared.units.snapshot();

        let foo = snap.iter().find(|e| e.name == "foo").unwrap();
        assert_eq!(foo.value, 22.0);

        // Snapshot is stable (insertion order). Ensure defaults exist and `foo` is at the end.
        assert_eq!(snap.first().unwrap().name, "U");
        assert_eq!(snap.last().unwrap().name, "foo");
    }
}

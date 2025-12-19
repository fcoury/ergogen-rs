use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::eval::eval_in_context;
use crate::expr::ScalarExpr;

#[derive(Debug, Clone)]
pub struct Units {
    map: IndexMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnitEntry {
    pub name: String,
    pub value: f64,
}

impl Units {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<f64> {
        self.map.get(key).copied()
    }

    #[must_use]
    pub fn vars(&self) -> &IndexMap<String, f64> {
        &self.map
    }

    pub fn eval(&self, key: &str, expr: &str) -> Result<f64, Error> {
        eval_in_context(key, expr, &self.map)
    }

    #[must_use]
    pub fn with_extra_vars(&self, extras: impl IntoIterator<Item = (String, f64)>) -> Self {
        let mut map = self.map.clone();
        for (k, v) in extras {
            map.insert(k, v);
        }
        Self { map }
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<UnitEntry> {
        self.map
            .iter()
            .map(|(k, v)| UnitEntry {
                name: k.clone(),
                value: *v,
            })
            .collect()
    }

    pub fn parse(
        units: Option<&IndexMap<String, ScalarExpr>>,
        variables: Option<&IndexMap<String, ScalarExpr>>,
    ) -> Result<Self, Error> {
        let mut merged = default_units();
        merge_into(&mut merged, units);
        merge_into(&mut merged, variables);

        let mut resolved: IndexMap<String, f64> = IndexMap::new();
        for (key, raw) in merged {
            let value = eval_scalar_expr(&key, &raw, &resolved)?;
            resolved.insert(key, value);
        }

        Ok(Self { map: resolved })
    }
}

fn default_units() -> IndexMap<String, ScalarExpr> {
    // Mirrors ../ergogen/src/units.js default_units and its insertion order.
    let mut m = IndexMap::new();
    m.insert("U".to_string(), ScalarExpr::Number(19.05));
    m.insert("u".to_string(), ScalarExpr::Number(19.0));
    m.insert("cx".to_string(), ScalarExpr::Number(18.0));
    m.insert("cy".to_string(), ScalarExpr::Number(17.0));
    m.insert("$default_stagger".to_string(), ScalarExpr::Number(0.0));
    m.insert(
        "$default_spread".to_string(),
        ScalarExpr::String("u".to_string()),
    );
    m.insert("$default_splay".to_string(), ScalarExpr::Number(0.0));
    m.insert(
        "$default_height".to_string(),
        ScalarExpr::String("u-1".to_string()),
    );
    m.insert(
        "$default_width".to_string(),
        ScalarExpr::String("u-1".to_string()),
    );
    m.insert(
        "$default_padding".to_string(),
        ScalarExpr::String("u".to_string()),
    );
    m.insert("$default_autobind".to_string(), ScalarExpr::Number(10.0));
    m
}

fn merge_into(
    target: &mut IndexMap<String, ScalarExpr>,
    from: Option<&IndexMap<String, ScalarExpr>>,
) {
    let Some(from) = from else { return };
    for (k, v) in from {
        if matches!(v, ScalarExpr::String(s) if s == "$unset") {
            target.shift_remove(k);
            continue;
        }
        if target.contains_key(k) {
            // Overwriting an existing key must *not* change insertion order (JS behavior).
            // IndexMap keeps order when overwriting.
            target.insert(k.clone(), v.clone());
        } else {
            target.insert(k.clone(), v.clone());
        }
    }
}

fn eval_scalar_expr(
    key: &str,
    raw: &ScalarExpr,
    vars: &IndexMap<String, f64>,
) -> Result<f64, Error> {
    match raw {
        ScalarExpr::Number(n) => Ok(*n),
        ScalarExpr::String(expr) => eval_expr(key, expr, vars),
    }
}

fn eval_expr(key: &str, expr: &str, vars: &IndexMap<String, f64>) -> Result<f64, Error> {
    eval_in_context(key, expr, vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RawConfig;

    #[test]
    fn default_units_match_upstream() {
        let units = Units::parse(None, None).unwrap();
        assert_eq!(units.get("U"), Some(19.05));
        assert_eq!(units.get("u"), Some(19.0));
        assert_eq!(units.get("cx"), Some(18.0));
        assert_eq!(units.get("cy"), Some(17.0));
        assert_eq!(units.get("$default_stagger"), Some(0.0));
        assert_eq!(units.get("$default_spread"), Some(19.0));
        assert_eq!(units.get("$default_height"), Some(18.0));
        assert_eq!(units.get("$default_width"), Some(18.0));
        assert_eq!(units.get("$default_padding"), Some(19.0));
        assert_eq!(units.get("$default_autobind"), Some(10.0));
    }

    #[test]
    fn overrides_and_appends_evaluate_in_order() {
        let cfg = RawConfig {
            units: Some(IndexMap::from([
                ("u".to_string(), ScalarExpr::Number(20.0)),
                ("cx".to_string(), ScalarExpr::String("u-1".to_string())),
            ])),
            variables: Some(IndexMap::from([(
                "foo".to_string(),
                ScalarExpr::String("u+2".to_string()),
            )])),
        };

        let units = Units::parse(cfg.units.as_ref(), cfg.variables.as_ref()).unwrap();
        assert_eq!(units.get("u"), Some(20.0));
        assert_eq!(units.get("cx"), Some(19.0));
        assert_eq!(units.get("foo"), Some(22.0));

        let snap = units.snapshot();
        let idx_u = snap.iter().position(|e| e.name == "u").unwrap();
        let idx_cx = snap.iter().position(|e| e.name == "cx").unwrap();
        let idx_foo = snap.iter().position(|e| e.name == "foo").unwrap();
        assert!(idx_u < idx_cx);
        assert!(idx_cx < idx_foo);
    }

    #[test]
    fn expression_table_is_deterministic() {
        let base = Units::parse(None, None).unwrap();
        let vars = &base.map;

        let cases: [(&str, f64); 6] = [
            ("u", 19.0),
            ("U", 19.05),
            ("u-1", 18.0),
            ("(u-1)*2", 36.0),
            ("cx + cy", 35.0),
            ("$default_height + 1", 19.0),
        ];

        for (expr, expected) in cases {
            let got = eval_expr("<table>", expr, vars).unwrap();
            assert_eq!(got, expected, "expr={expr}");
        }
    }

    #[test]
    fn expressions_can_reference_dollar_prefixed_units() {
        let cfg = RawConfig {
            units: None,
            variables: Some(IndexMap::from([(
                "bar".to_string(),
                ScalarExpr::String("$default_height + 2".to_string()),
            )])),
        };
        let units = Units::parse(cfg.units.as_ref(), cfg.variables.as_ref()).unwrap();
        assert_eq!(units.get("bar"), Some(20.0));
    }
}

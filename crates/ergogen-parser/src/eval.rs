use indexmap::IndexMap;
use std::str::FromStr;

use crate::error::Error;

pub fn eval_in_context(key: &str, expr: &str, vars: &IndexMap<String, f64>) -> Result<f64, Error> {
    let normalized = insert_implicit_multiplication(expr);
    let (rewritten, ctx) = rewrite_expr_and_context(key, &normalized, vars)?;
    let parsed = meval::Expr::from_str(&rewritten).map_err(|_| Error::InvalidExpression {
        key: key.to_string(),
        expr: expr.to_string(),
    })?;
    parsed.eval_with_context(ctx).map_err(|e| Error::Eval {
        key: key.to_string(),
        message: format!("{e}"),
    })
}

fn rewrite_expr_and_context(
    key: &str,
    expr: &str,
    vars: &IndexMap<String, f64>,
) -> Result<(String, meval::Context<'static>), Error> {
    let mut rewritten = String::with_capacity(expr.len());
    let mut ctx = meval::Context::new();

    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if is_ident_start(ch) {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let c = bytes[i] as char;
                if is_ident_continue(c) {
                    i += 1;
                } else {
                    break;
                }
            }

            let ident = &expr[start..i];
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            let is_fn_call = j < bytes.len() && bytes[j] as char == '(';

            if is_fn_call {
                rewritten.push_str(ident);
                continue;
            }

            let Some(value) = vars.get(ident).copied() else {
                if is_allowed_builtin_constant(ident) {
                    rewritten.push_str(ident);
                    continue;
                }
                return Err(Error::UnknownVariable {
                    key: key.to_string(),
                    name: ident.to_string(),
                });
            };
            let safe = sanitize_ident(ident);
            ctx.var(safe.clone(), value);
            rewritten.push_str(&safe);
        } else {
            rewritten.push(ch);
            i += 1;
        }
    }

    Ok((rewritten, ctx))
}

fn insert_implicit_multiplication(expr: &str) -> String {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Tok {
        Number,
        Ident,
        LParen,
        RParen,
        Other,
    }

    fn is_ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_' || c == '$'
    }

    fn is_ident_continue(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }

    fn is_known_function(name: &str) -> bool {
        matches!(
            name,
            "sqrt"
                | "sin"
                | "cos"
                | "tan"
                | "asin"
                | "acos"
                | "atan"
                | "abs"
                | "floor"
                | "ceil"
                | "round"
        )
    }

    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len() + 8);
    let mut i = 0;
    let mut prev_tok: Option<Tok> = None;
    let mut prev_ident: Option<String> = None;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            out.push(c);
            i += 1;
            continue;
        }

        let (tok, text, ident_name) = if c.is_ascii_digit() || c == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let mut s = chars[start..i].iter().collect::<String>();
            // `meval` doesn't accept floats like `.5`, but upstream configs often use `.5u` / `-.5u`.
            if s.starts_with('.') {
                s.insert(0, '0');
            }
            (Tok::Number, s, None)
        } else if is_ident_start(c) {
            let start = i;
            i += 1;
            while i < chars.len() && is_ident_continue(chars[i]) {
                i += 1;
            }
            let s = chars[start..i].iter().collect::<String>();
            (Tok::Ident, s.clone(), Some(s))
        } else if c == '(' {
            i += 1;
            (Tok::LParen, "(".to_string(), None)
        } else if c == ')' {
            i += 1;
            (Tok::RParen, ")".to_string(), None)
        } else {
            i += 1;
            (Tok::Other, c.to_string(), None)
        };

        let should_insert_mul = match (prev_tok, tok) {
            (Some(Tok::Number | Tok::Ident | Tok::RParen), Tok::Ident | Tok::Number) => true,
            (Some(Tok::Number | Tok::RParen), Tok::LParen) => true,
            (Some(Tok::Ident), Tok::LParen) => {
                // Treat as multiplication unless it looks like a known function call.
                match prev_ident.as_deref() {
                    Some(prev) => !is_known_function(prev),
                    None => true,
                }
            }
            _ => false,
        };

        if should_insert_mul {
            out.push('*');
        }

        out.push_str(&text);
        prev_tok = Some(tok);
        prev_ident = ident_name;
    }

    out
}

fn is_allowed_builtin_constant(ident: &str) -> bool {
    matches!(ident, "pi" | "e")
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

fn sanitize_ident(raw: &str) -> String {
    let mut out = String::from("v_");
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
            out.push_str(&format!("{:x}", c as u32));
            out.push('_');
        }
    }
    out
}

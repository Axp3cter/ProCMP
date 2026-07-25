//! `{token}` expansion for `output` and `header`.

use indexmap::IndexMap;

use crate::error::{Error, Result};

/// `{{` and `}}` are literal braces. An unknown, empty or unclosed token is an error.
pub fn expand(template: &str, tokens: &IndexMap<String, String>) -> Result<String> {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => {
                out.push('{');
                i += 2;
            }
            b'}' if bytes.get(i + 1) == Some(&b'}') => {
                out.push('}');
                i += 2;
            }
            b'{' => {
                let start = i + 1;
                let end = template[start..]
                    .find('}')
                    .map(|offset| start + offset)
                    .ok_or_else(|| Error::UnterminatedToken(template.into()))?;

                let token = &template[start..end];
                let value = tokens.get(token).ok_or_else(|| {
                    let mut names: Vec<_> = tokens.keys().map(|k| format!("{{{k}}}")).collect();
                    names.sort();
                    Error::UnknownToken(token.into(), names.join(", "))
                })?;

                if value.is_empty() {
                    return Err(Error::EmptyToken(token.into()));
                }

                out.push_str(value);
                i = end + 1;
            }
            // An unpaired `}` is not ambiguous the way `{` is.
            b'}' => {
                out.push('}');
                i += 1;
            }
            _ => {
                let start = i;
                while i < bytes.len() && bytes[i] != b'{' && bytes[i] != b'}' {
                    i += 1;
                }
                out.push_str(&template[start..i]);
            }
        }
    }

    Ok(out)
}

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Whether `name` can appear as a global in Luau source.
pub fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();

    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
        && !KEYWORDS.contains(&name)
}

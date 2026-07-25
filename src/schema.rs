//! Editor-facing schema emission.
//!
//! Both outputs are generated from the [`Manifest`] type the parser uses, so neither
//! can describe something ProCMP would reject. Adding a manifest field is one edit.

use serde_json::Value;

use crate::manifest::Manifest;

/// Emits the manifest JSON Schema, for editors and CI validation.
pub fn json() -> String {
    serde_json::to_string_pretty(&schema()).expect("a generated schema is always encodable")
}

/// Emits Luau type definitions for `pcmp.luau`, giving `luau-lsp` completion and type
/// errors inside a manifest.
pub fn luau() -> String {
    let schema = schema();
    let mut out = String::from(
        "--!strict\n\
         --- Type definitions for ProCMP manifests. Emitted by `pcmp schema --luau`.\n\n",
    );

    if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
        // Sorted rather than left in schemars' insertion order, so regenerating after
        // an unrelated field moves does not rewrite the whole file.
        let mut names: Vec<&String> = defs.keys().collect();
        names.sort();

        for name in names {
            let Some(body) = defs.get(name) else { continue };
            out.push_str(&format!("export type {name} = {}\n\n", type_of(body)));
        }
    }
    out.push_str(&format!("export type Manifest = {}\n\n", type_of(&schema)));

    // Hand-written on purpose: this describes the globals `crate::load` installs, not
    // the manifest shape, so there is nothing to derive it from.
    out.push_str(
        "--- The globals available inside a manifest.\n\
         export type Api = {\n\
         \t--- Reads an environment variable. Errors when it is not set.\n\
         \tenv: (name: string) -> string,\n\
         \t--- Reads an environment variable, with an explicit fallback.\n\
         \tenvOr: (name: string, fallback: string) -> string,\n\
         }\n\n\
         declare pcmp: Api\n\n\
         return nil\n",
    );

    out
}

fn schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Manifest))
        .expect("a generated schema is always encodable")
}

/// Renders one schema node as a Luau type.
///
/// Handles exactly the shapes `schemars` produces for [`Manifest`]: references,
/// nullable unions, string enums, arrays, string-keyed maps and primitives. Anything
/// else becomes `any` rather than a plausible-looking guess.
fn type_of(node: &Value) -> String {
    if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
        // A reference with nothing after its last separator would name a type `` and
        // emit `export type  = ...`, which does not parse.
        return match reference.rsplit('/').next() {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => "any".to_owned(),
        };
    }

    // `oneOf` carries string enums: each branch is a single `const`.
    if let Some(branches) = node.get("oneOf").and_then(Value::as_array) {
        let literals: Vec<String> = branches
            .iter()
            .filter_map(|b| b.get("const").and_then(Value::as_str))
            .map(|c| format!("\"{c}\""))
            .collect();
        if !literals.is_empty() {
            return literals.join(" | ");
        }
    }

    // `anyOf` is either an untagged union or an optional wrapping one branch.
    if let Some(branches) = node.get("anyOf").and_then(Value::as_array) {
        let nullable = branches.iter().any(is_null);
        let rendered: Vec<String> = branches
            .iter()
            .filter(|b| !is_null(b))
            .map(type_of)
            .collect();

        let joined = match rendered.len() {
            0 => "any".to_owned(),
            1 => rendered.into_iter().next().unwrap_or_default(),
            _ => rendered.join(" | "),
        };
        return if nullable { optional(&joined) } else { joined };
    }

    match node.get("type") {
        // `["array", "null"]` is an optional *anything*, not just an optional
        // primitive. This is the shape `Option<Vec<T>>` and `Option<Struct>` produce.
        Some(Value::Array(kinds)) => {
            let nullable = kinds.iter().any(|k| k == "null");
            let base = kinds
                .iter()
                .filter(|k| *k != "null")
                .find_map(Value::as_str)
                .map_or_else(|| "any".to_owned(), |kind| named(kind, node));
            if nullable { optional(&base) } else { base }
        }

        Some(Value::String(kind)) => named(kind, node),

        _ => "any".to_owned(),
    }
}

/// Renders one type name. `array` and `object` need the node too, because their shape
/// lives beside the name rather than inside it.
fn named(kind: &str, node: &Value) -> String {
    match kind {
        "array" => format!("{{ {} }}", node.get("items").map_or("any".into(), type_of)),
        "object" => object_of(node),
        other => primitive(other),
    }
}

/// Renders an object as a record of known fields, or as a map when the keys are open.
fn object_of(node: &Value) -> String {
    if let Some(values) = node.get("additionalProperties")
        && values.is_object()
    {
        return format!("{{ [string]: {} }}", type_of(values));
    }

    let Some(properties) = node.get("properties").and_then(Value::as_object) else {
        return "{ [string]: any }".to_owned();
    };

    let required: Vec<&str> = node
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut fields = String::new();
    for (name, body) in properties {
        let mut rendered = type_of(body);
        if !required.contains(&name.as_str()) {
            rendered = optional(&rendered);
        }
        fields.push_str(&format!("\t{}: {rendered},\n", key(name)));
    }

    format!("{{\n{fields}}}")
}

/// Luau words that cannot appear as a bare field name.
const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Quotes a key that is not a bare Luau identifier.
///
/// A keyword is quoted too. A manifest field named `end` is legal JSON and legal Rust,
/// and emitting it bare would produce definitions that do not parse.
fn key(name: &str) -> String {
    let bare = !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&name);

    if bare {
        name.to_owned()
    } else {
        format!("[\"{name}\"]")
    }
}

/// Makes a type optional without producing `T??` or an ambiguous `A | B?`.
fn optional(rendered: &str) -> String {
    if rendered.ends_with('?') {
        rendered.to_owned()
    } else if rendered.contains(" | ") {
        format!("({rendered})?")
    } else {
        format!("{rendered}?")
    }
}

fn is_null(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("null")
}

fn primitive(kind: &str) -> String {
    match kind {
        "string" => "string",
        "boolean" => "boolean",
        "integer" | "number" => "number",
        _ => "any",
    }
    .to_owned()
}

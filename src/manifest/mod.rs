//! The config surface, as written by a user.
//!
//! ProCMP owns eight profile fields. Everything that configures a transformation lives
//! under [`Profile::darklua`] and is darklua's own format, deserialised by darklua, so
//! no capability of the linked version is unreachable.

mod discover;
mod luau;

pub use discover::{CANDIDATES, Format, Loaded, discover, load, parse};
pub use luau::{Env, REVOKED};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    /// Pointer for editor validation. Accepted and ignored.
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Named strings every profile starts from.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,

    #[serde(default)]
    pub profiles: IndexMap<String, Profile>,

    #[serde(default)]
    pub matrix: IndexMap<String, Matrix>,
}

/// Every overridable field is an [`Option`], collections included: inheritance has to
/// tell "not declared" from "declared empty". `vars` and `define` accumulate instead.
#[derive(Debug, Clone, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Profile {
    /// A template that builds nothing, exempt from `entry` and `output`.
    #[serde(
        default,
        rename = "abstract",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub is_abstract: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// One file, or a directory processed into a directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,

    /// Token template. No default: a guessed location produces artifacts nobody knows
    /// to look for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Extra directories whose contents count as build inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,

    /// Globs excluded from that input set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,

    /// Each becomes a `{token}` and a `PCMP_<NAME>` constant.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,

    /// Compile-time constants: injected as globals, then folded away.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub define: IndexMap<String, Define>,

    /// Written above each artifact after darklua runs, because the `dense` and
    /// `readable` generators discard comments, Luau directives included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<String>>,

    /// darklua spells this as a map and takes the first match, but a Luau table
    /// iterates in hash order, so a map here would let the format decide which pattern
    /// wins. The one place darklua's own shape is not passed through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaders: Option<Vec<Loader>>,

    /// darklua's configuration, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub darklua: Option<Map<String, Value>>,
}

/// The cartesian product of its axes, one task per combination.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Matrix {
    pub base: String,
    pub axes: IndexMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub define: IndexMap<String, Define>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Define {
    Bool(bool),
    /// Infinity and NaN are rejected during resolution.
    Number(f64),
    Text(String),
}

impl Define {
    /// Type-tagged, so `true` and `"true"` never share a cache key.
    pub fn tagged(&self) -> String {
        match self {
            Self::Bool(v) => format!("bool:{v}"),
            Self::Number(v) => format!("number:{v:?}"),
            Self::Text(v) => format!("string:{v}"),
        }
    }

    /// Parses a `--define` value. Only a number that survives a round trip is read as
    /// one, so `007` stays the string it was written as.
    pub fn parse(text: &str) -> Self {
        match text {
            "true" => Self::Bool(true),
            "false" => Self::Bool(false),
            other => match other.parse::<f64>() {
                Ok(n) if n.is_finite() && format!("{n}") == other => Self::Number(n),
                _ => Self::Text(other.to_owned()),
            },
        }
    }
}

/// Untyped on purpose: darklua owns the rule vocabulary and validates it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Rule {
    Named(String),
    Detailed(Map<String, Value>),
}

impl Rule {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::Detailed(map) => map.get("rule").and_then(Value::as_str),
        }
    }

    pub fn json(&self) -> Value {
        match self {
            Self::Named(name) => Value::String(name.clone()),
            Self::Detailed(map) => Value::Object(map.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Loader {
    pub pattern: String,
    /// `copy`, `json`, `toml`, `yaml`, `string`, `buffer`, `bytes`, `skip`, or an
    /// encoded form such as `string/base64`.
    #[serde(rename = "use")]
    pub loader: String,
}

impl Manifest {
    /// Sorts every map into key order, which is what makes the formats interchangeable.
    pub fn normalise(&mut self) {
        self.vars.sort_keys();
        self.profiles.sort_keys();
        self.matrix.sort_keys();

        for profile in self.profiles.values_mut() {
            profile.vars.sort_keys();
            profile.define.sort_keys();

            // Neither list carries meaning in its order. `loaders` is not here: its
            // order decides which pattern wins.
            for list in [profile.sources.as_mut(), profile.ignore.as_mut()]
                .into_iter()
                .flatten()
            {
                list.sort();
                list.dedup();
            }

            // A Luau table reaches serde in hash order, so an unsorted block would
            // serialise differently between runs and give one config two cache keys.
            if let Some(darklua) = profile.darklua.as_mut() {
                darklua.sort_keys();
                darklua.values_mut().for_each(sort_keys);
            }
        }

        for matrix in self.matrix.values_mut() {
            matrix.axes.sort_keys();
            matrix.define.sort_keys();
            matrix.vars.sort_keys();
        }
    }
}

fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.sort_keys();
            map.values_mut().for_each(sort_keys);
        }
        Value::Array(items) => items.iter_mut().for_each(sort_keys),
        _ => {}
    }
}

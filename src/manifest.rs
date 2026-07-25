//! The config surface, as written by a user.
//!
//! The raw shape, before `extends` runs. This is the only layer where an optional
//! field means "inherit". [`crate::plan`] resolves these into types where nothing is
//! optional.
//!
//! ProCMP owns eight fields. Everything that configures a transformation lives under
//! [`Profile::darklua`] and is darklua's own configuration format, deserialised by
//! darklua, so no capability of the linked version is unreachable from a manifest and
//! no field here needs updating when darklua gains one.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A complete manifest, in any of the supported formats.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    /// Pointer for editor validation. Accepted and ignored.
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Named strings every profile starts from.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,

    /// Key order is normalised at load time: Luau iterates tables in hash order and
    /// JSON preserves document order, and neither should reach the plan.
    #[serde(default)]
    pub profiles: IndexMap<String, Profile>,

    /// Matrix expansions, one task per axis combination.
    #[serde(default)]
    pub matrix: IndexMap<String, Matrix>,
}

/// One named way of building the project, before inheritance is applied.
///
/// Every overridable field is an [`Option`], including the collections: inheritance has
/// to tell "not declared" from "declared empty", or a profile could never clear a list
/// it inherited. `vars` and `define` are the exceptions, and accumulate.
#[derive(Debug, Clone, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Profile {
    /// A template that builds nothing itself, exempt from `entry` and `output`.
    #[serde(
        default,
        rename = "abstract",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub is_abstract: bool,

    /// Profile to inherit from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// What darklua processes: one file, or a directory processed into a directory.
    /// Required once inheritance has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,

    /// Token template for the destination. Required once inheritance has run. There is
    /// no default, because a guessed location produces artifacts nobody knows to look
    /// for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Extra directories whose contents count as build inputs.
    ///
    /// The manifest's own directory is always one. Add a sibling package here and
    /// editing it invalidates the cache and wakes `pcmp watch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,

    /// Globs excluded from that input set, matched against paths relative to each root.
    ///
    /// Outputs, the cache directory and `.git` are excluded already. This is for
    /// vendored trees large enough that hashing them is the slow part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,

    /// Named strings, merged over the manifest's. Each becomes a `{token}` in `output`
    /// and `header`, and a `PCMP_<NAME>` constant.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,

    /// Compile-time constants: injected as globals, then folded away.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub define: IndexMap<String, Define>,

    /// Lines written verbatim at the top of each artifact, after darklua runs.
    ///
    /// Written by ProCMP rather than by a darklua rule, because the `dense` and
    /// `readable` generators discard comments. That includes Luau directives such as
    /// `--!native`, which must survive into a minified build.
    ///
    /// Tokens are expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<String>>,

    /// How to treat files darklua would otherwise ignore.
    ///
    /// darklua spells this as a map, and takes the first pattern that matches. A Luau
    /// table iterates in hash order, so a map here would let the manifest format decide
    /// which pattern wins. An ordered list is the one place ProCMP does not pass
    /// darklua's own shape through, and this is why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaders: Option<Vec<Loader>>,

    /// darklua's configuration, verbatim: `generator`, `rules`, `bundle`,
    /// `apply_to_files`, `skip_files`, `lua_extension`, and anything the linked version
    /// adds. Deserialised by darklua, so its documentation is the reference.
    ///
    /// `pcmp explain` prints what this resolves to, which is a valid `.darklua.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub darklua: Option<Map<String, Value>>,
}

/// A value injected into source as a literal.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Define {
    /// A boolean. Foldable, so it can eliminate a branch entirely.
    Bool(bool),
    /// A finite number. Infinity and NaN are rejected during resolution.
    Number(f64),
    /// A string. Injected quoted, so it stays a string in the output.
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

    /// Parses a `--define KEY=VALUE` value. `true`, `false` and any number are read as
    /// such. Everything else stays a string, which is what a shell mostly produces.
    ///
    /// Only a number that survives a round trip is read as one, so `007` and `1_000`
    /// stay the strings they were written as rather than becoming `7` and `1000`.
    pub fn parse(text: &str) -> Self {
        match text {
            "true" => Self::Bool(true),
            "false" => Self::Bool(false),
            other => match other.parse::<f64>() {
                Ok(number) if number.is_finite() && format!("{number}") == other => {
                    Self::Number(number)
                }
                _ => Self::Text(other.to_owned()),
            },
        }
    }
}

/// Untyped on purpose: darklua owns the rule vocabulary, and validation is delegated
/// to its deserialiser, so the accepted set matches the linked version exactly.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Rule {
    /// A rule with no parameters.
    Named(String),
    /// A rule table, named by its `rule` key.
    Detailed(Map<String, Value>),
}

impl Rule {
    /// Returns the rule's name, or [`None`] if a parameterised rule omits `rule`.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::Detailed(map) => map.get("rule").and_then(Value::as_str),
        }
    }

    /// Returns the JSON darklua will deserialise this from.
    pub fn json(&self) -> Value {
        match self {
            Self::Named(name) => Value::String(name.clone()),
            Self::Detailed(map) => Value::Object(map.clone()),
        }
    }
}

/// One glob-to-loader mapping.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Loader {
    /// Glob matched against each file's path.
    pub pattern: String,
    /// A darklua loader: `copy`, `json`, `toml`, `yaml`, `json_lines`, `string`,
    /// `buffer`, `bytes`, `skip`, or one with an encoding suffix such as
    /// `string/base64` or `buffer/zstd`.
    #[serde(rename = "use")]
    pub loader: String,
}

/// The cartesian product of its axes, one task per combination.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Matrix {
    /// Profile each generated task starts from.
    pub base: String,
    /// Axis name to values. The product is the task list.
    pub axes: IndexMap<String, Vec<String>>,
    /// Output template. Axis values are available as tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Extra defines applied to every generated task.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub define: IndexMap<String, Define>,
    /// Extra vars applied to every generated task.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,
}

impl Manifest {
    /// Sorts every map into key order, which is what makes the manifest formats
    /// interchangeable. Called once, at the end of loading.
    pub fn normalise(&mut self) {
        self.vars.sort_keys();
        self.profiles.sort_keys();
        self.matrix.sort_keys();

        for profile in self.profiles.values_mut() {
            profile.vars.sort_keys();
            profile.define.sort_keys();

            // Neither list carries meaning in its order, so two manifests naming the
            // same roots differently should not resolve to two cache keys. `loaders`
            // is deliberately not here: its order decides which pattern wins.
            for list in [profile.sources.as_mut(), profile.ignore.as_mut()]
                .into_iter()
                .flatten()
            {
                list.sort();
                list.dedup();
            }

            // A Luau table reaches serde in hash order, so an unsorted `darklua` block
            // would serialise differently on each run and thrash the cache. `loaders`
            // is the one map whose order carries meaning, and it is not in here.
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

/// Sorts every object key in a JSON tree, in place.
///
/// Recurses through arrays as well as objects, at any depth. A single object left in
/// hash order anywhere under `darklua` would serialise differently between runs and
/// give one configuration two cache keys.
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

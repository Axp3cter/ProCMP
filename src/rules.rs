//! Assembling the darklua rule list for a task.
//!
//! ProCMP adds no rule vocabulary of its own. A manifest names darklua rules directly,
//! in the order it wants them, and that order is preserved — `pcmp check` reports an
//! ordering that will not do what it looks like, rather than silently rewriting it.
//!
//! The one thing ProCMP contributes is injection: a `define` becomes an
//! `inject_global_value` rule, placed first because nothing downstream can fold a
//! value that has not been substituted yet.

use indexmap::IndexMap;
use serde_json::{Map, Number, Value};

use crate::manifest::{Define, Rule};

/// The rule that turns a define into a literal.
pub const INJECT: &str = "inject_global_value";

/// Builds one injection rule per define.
///
/// `defines` arrives key-sorted from [`crate::manifest::Manifest::normalise`], so the
/// emitted order does not depend on how the manifest was written.
pub fn injections(defines: &IndexMap<String, Define>) -> Vec<Rule> {
    defines
        .iter()
        .map(|(identifier, value)| {
            let mut map = Map::new();
            map.insert("rule".into(), INJECT.into());
            map.insert("identifier".into(), identifier.as_str().into());
            map.insert(
                "value".into(),
                match value {
                    Define::Bool(v) => Value::Bool(*v),
                    Define::Number(v) => Number::from_f64(*v)
                        .map(Value::Number)
                        .expect("resolution rejects non-finite defines before reaching here"),
                    Define::Text(v) => Value::String(v.clone()),
                },
            );
            Rule::Detailed(map)
        })
        .collect()
}

/// The rule list for a task, or [`None`] to let darklua apply its own defaults.
///
/// Declaring no rules and no defines is the only case that defers: once a define
/// exists it has to be injected, and a rule list cannot be half-specified, so
/// darklua's defaults are read back and the injections placed ahead of them.
pub fn assemble(
    defines: &IndexMap<String, Define>,
    declared: Option<&[Rule]>,
) -> Option<Vec<Rule>> {
    let injections = injections(defines);

    match (declared, injections.is_empty()) {
        (None, true) => None,
        (None, false) => Some([injections, darklua_defaults()].concat()),
        (Some(rules), _) => Some([injections, rules.to_vec()].concat()),
    }
}

/// darklua's own default rules, read from the linked version rather than copied.
fn darklua_defaults() -> Vec<Rule> {
    darklua_core::rules::get_default_rules()
        .iter()
        .filter_map(|rule| serde_json::to_value(rule.as_ref()).ok())
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}

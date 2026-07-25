//! Assembling the darklua rule list.
//!
//! ProCMP adds no rule vocabulary. The one thing it contributes is injection: a
//! `define` becomes an `inject_global_value` rule, placed first because nothing
//! downstream can fold a value that has not been substituted yet.

use std::sync::OnceLock;

use indexmap::IndexMap;
use serde_json::{Map, Number, Value};

use crate::manifest::{Define, Rule};

pub const INJECT: &str = "inject_global_value";

/// `defines` arrives key-sorted, so the emitted order does not depend on how the
/// manifest was written.
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
                        .expect("resolution rejects non-finite defines"),
                    Define::Text(v) => Value::String(v.clone()),
                },
            );
            Rule::Detailed(map)
        })
        .collect()
}

/// [`None`] lets darklua apply its own defaults. Declaring no rules and no defines is
/// the only case that defers: once a define exists it has to be injected, and a rule
/// list cannot be half-specified.
pub fn assemble(
    defines: &IndexMap<String, Define>,
    declared: Option<&[Rule]>,
) -> Option<Vec<Rule>> {
    let injections = injections(defines);

    match (declared, injections.is_empty()) {
        (None, true) => None,
        (None, false) => Some([injections.as_slice(), defaults()].concat()),
        (Some(rules), _) => Some([injections.as_slice(), rules].concat()),
    }
}

/// Read from the linked version rather than copied, once for the process.
fn defaults() -> &'static [Rule] {
    static DEFAULTS: OnceLock<Vec<Rule>> = OnceLock::new();

    DEFAULTS.get_or_init(|| {
        darklua_core::rules::get_default_rules()
            .iter()
            .map(|rule| {
                let value = serde_json::to_value(rule.as_ref()).expect("a darklua rule serialises");
                serde_json::from_value(value).expect("a darklua rule deserialises")
            })
            .collect()
    })
}

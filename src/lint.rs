//! Checks a type checker cannot express.
//!
//! Two kinds: rule orderings that will not do what the manifest appears to intend, and
//! profiles that are declared but do nothing. Both report rather than repair, because
//! [`crate::rules::assemble`] preserves the order a manifest writes and a lint that
//! silently rewrote it would make `pcmp explain` a lie.
//!
//! Deliberately small. A check no input can trigger reads as coverage while providing
//! none, so every one here has a manifest that fires it.

use std::collections::BTreeMap;

use crate::diag::Diag;
use crate::digest::{self, Digest};
use crate::manifest::{Manifest, Rule};
use crate::plan::{Graph, Task};
use crate::rules;

/// Constant folding scheduled before any value injection.
pub const FOLD_BEFORE_INJECT: &str = "fold-before-inject";
/// Branch elimination scheduled before constant folding.
pub const BRANCH_BEFORE_FOLD: &str = "branch-before-fold";
/// An abstract profile nothing extends.
pub const UNUSED_PROFILE: &str = "unused-profile";
/// Two profiles declared with identical configuration.
pub const IDENTICAL_PROFILES: &str = "identical-profiles";

const FOLD: &str = "compute_expression";
const BRANCH: &str = "remove_unused_if_branch";

/// One pair of rules whose relative order changes what a build produces.
struct Ordered {
    earlier: &'static str,
    later: &'static str,
    code: &'static str,
    /// What goes wrong when they are the other way round.
    why: &'static str,
}

/// The dependency chain darklua's own documentation calls out: a value has to be
/// injected before it can be folded, and folded before a branch on it can be removed.
///
/// Nothing else is listed. darklua's default rule list is itself a valid ordering, and
/// a lint stricter than the tool it lints for would reject working manifests.
const PIPELINE: &[Ordered] = &[
    Ordered {
        earlier: rules::INJECT,
        later: FOLD,
        code: FOLD_BEFORE_INJECT,
        why: "folding cannot see a value substituted after it, so the define does nothing",
    },
    Ordered {
        earlier: FOLD,
        later: BRANCH,
        code: BRANCH_BEFORE_FOLD,
        why: "a branch is only removable once its condition has folded to a constant",
    },
];

/// Every finding this module can produce, unsorted.
pub fn run(manifest: &Manifest, graph: &Graph) -> Vec<Diag> {
    let mut diags = Vec::new();

    for task in &graph.tasks {
        ordering(task, &mut diags);
    }
    orphans(manifest, &mut diags);
    duplicates(manifest, &mut diags);

    // Left unsorted: `pcmp check` merges this with the resolver's findings and sorts
    // once, so sorting here would only be thrown away.
    diags
}

/// Walks [`PIPELINE`] against one task's rule list.
///
/// A rule listed twice is not a finding: running one pass after another has produced
/// new foldable code is a technique, not a mistake.
fn ordering(task: &Task, diags: &mut Vec<Diag>) {
    let Some(rules) = &task.rules else {
        return;
    };

    let names: Vec<&str> = rules.iter().filter_map(Rule::name).collect();

    for pair in PIPELINE {
        // First occurrence of the later rule against the last of the earlier one: the
        // ordering only holds if every `earlier` precedes every `later`.
        let (Some(later), Some(earlier)) = (
            names.iter().position(|n| *n == pair.later),
            names.iter().rposition(|n| *n == pair.earlier),
        ) else {
            continue;
        };

        if later < earlier {
            diags.push(
                Diag::deny(
                    pair.code,
                    format!(
                        "task `{}`: `{}` runs before `{}`",
                        task.id, pair.later, pair.earlier
                    ),
                )
                .help(format!(
                    "list every `{}` ahead of `{}`\n  {}",
                    pair.earlier, pair.later, pair.why
                )),
            );
        }
    }
}

/// Abstract only: a concrete profile always becomes a task, so widening this check
/// would make it unfireable.
fn orphans(manifest: &Manifest, diags: &mut Vec<Diag>) {
    for (name, profile) in &manifest.profiles {
        if !profile.is_abstract {
            continue;
        }

        let extended = manifest
            .profiles
            .values()
            .any(|p| p.extends.as_deref() == Some(name.as_str()));
        let is_base = manifest.matrix.values().any(|m| &m.base == name);

        if !extended && !is_base {
            diags.push(
                Diag::warn(
                    UNUSED_PROFILE,
                    format!("abstract profile `{name}` is never extended or used as a matrix base"),
                )
                .help("remove it, or drop `abstract` so it builds"),
            );
        }
    }
}

/// Compares declarations, not resolved tasks: every task carries its own injected
/// `PCMP_PROFILE`, so no two are ever identical.
fn duplicates(manifest: &Manifest, diags: &mut Vec<Diag>) {
    let mut by_shape: BTreeMap<Digest, Vec<&str>> = BTreeMap::new();

    for (name, profile) in &manifest.profiles {
        // An abstract profile exists to be extended, so one that matches its own child
        // is the intended shape rather than a duplicate.
        if profile.is_abstract {
            continue;
        }

        let mut shape = profile.clone();
        // Two profiles differing only by where they write are the normal case, and so
        // are two differing only in the values their tokens carry.
        shape.output = None;
        shape.vars.clear();

        let rendered = serde_json::to_string(&shape).expect("a profile always serialises");
        by_shape.entry(digest::of(rendered)).or_default().push(name);
    }

    for names in by_shape.into_values().filter(|n| n.len() > 1) {
        diags.push(
            Diag::warn(
                IDENTICAL_PROFILES,
                format!(
                    "profiles {} are declared identically",
                    names
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .help("extract the shared settings into an `abstract` profile and `extends` it"),
        );
    }
}

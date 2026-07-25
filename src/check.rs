//! Checks a type checker cannot express. All report, none repair.

use std::collections::BTreeMap;

use crate::diag::Diag;
use crate::digest::{self, Digest};
use crate::manifest::{Manifest, Rule};
use crate::plan::{Graph, Task, rules};

pub const FOLD_BEFORE_INJECT: &str = "fold-before-inject";
pub const BRANCH_BEFORE_FOLD: &str = "branch-before-fold";
pub const UNUSED_PROFILE: &str = "unused-profile";
pub const IDENTICAL_PROFILES: &str = "identical-profiles";

const FOLD: &str = "compute_expression";
const BRANCH: &str = "remove_unused_if_branch";

struct Ordered {
    earlier: &'static str,
    later: &'static str,
    code: &'static str,
    why: &'static str,
}

/// The orderings darklua's own documentation calls out.
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

/// Unsorted: `check` merges these with the resolver's findings and sorts once.
pub fn run(manifest: &Manifest, graph: &Graph) -> Vec<Diag> {
    let mut diags = Vec::new();

    for task in &graph.tasks {
        ordering(task, &mut diags);
    }
    orphans(manifest, &mut diags);
    duplicates(manifest, &mut diags);

    diags
}

/// A rule listed twice is not a finding.
fn ordering(task: &Task, diags: &mut Vec<Diag>) {
    let Some(rules) = &task.rules else {
        return;
    };

    let names: Vec<&str> = rules.iter().filter_map(Rule::name).collect();

    for pair in PIPELINE {
        // First of the later against last of the earlier: the ordering holds only if
        // every `earlier` precedes every `later`.
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

/// Abstract only: a concrete profile always becomes a task.
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

/// Compares declarations, not tasks: every task carries its own `PCMP_PROFILE`.
fn duplicates(manifest: &Manifest, diags: &mut Vec<Diag>) {
    let mut by_shape: BTreeMap<Digest, Vec<&str>> = BTreeMap::new();

    for (name, profile) in &manifest.profiles {
        // An abstract profile matches its own child by design.
        if profile.is_abstract {
            continue;
        }

        let mut shape = profile.clone();
        // Excluded: differing only here is the normal case.
        shape.output = None;
        shape.vars.clear();

        let rendered = serde_json::to_string(&shape).expect("a profile always serialises");
        by_shape.entry(digest::of(rendered)).or_default().push(name);
    }

    for names in by_shape.into_values().filter(|n| n.len() > 1) {
        let listed = names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(", ");

        diags.push(
            Diag::warn(
                IDENTICAL_PROFILES,
                format!("profiles {listed} are declared identically"),
            )
            .help("extract the shared settings into an `abstract` profile and `extends` it"),
        );
    }
}

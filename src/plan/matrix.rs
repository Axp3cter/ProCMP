//! Expanding a matrix into one task per axis combination.

use indexmap::IndexMap;
use std::collections::BTreeSet;

use crate::diag::Diag;
use crate::manifest::{Manifest, Matrix};
use crate::path::AbsPath;

use super::inherit::flatten;
use super::{DUPLICATE_AXIS_VALUE, EMPTY_AXIS, Overrides, Task, UNKNOWN_MATRIX_BASE, known, task};

pub fn expand(
    manifest: &Manifest,
    name: &str,
    matrix: &Matrix,
    root: &AbsPath,
    overrides: &Overrides,
    tasks: &mut Vec<Task>,
    diags: &mut Vec<Diag>,
) {
    if !manifest.profiles.contains_key(&matrix.base) {
        diags.push(
            Diag::deny(
                UNKNOWN_MATRIX_BASE,
                format!("matrix `{name}` extends unknown profile `{}`", matrix.base),
            )
            .help(format!("known: {}", known(manifest))),
        );
        return;
    }

    // Every axis is reported, so one edit fixes them all.
    let mut usable = true;

    for (axis, values) in &matrix.axes {
        if values.is_empty() {
            diags.push(
                Diag::deny(
                    EMPTY_AXIS,
                    format!("matrix `{name}` axis `{axis}` has no values"),
                )
                .help("an empty axis expands to zero tasks"),
            );
            usable = false;
            continue;
        }

        // A repeat expands to two tasks with identical coordinates.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        if let Some(repeated) = values.iter().find(|value| !seen.insert(value.as_str())) {
            diags.push(
                Diag::deny(
                    DUPLICATE_AXIS_VALUE,
                    format!("matrix `{name}` axis `{axis}` lists `{repeated}` twice"),
                )
                .help("each value expands to one task, so a repeat adds nothing"),
            );
            usable = false;
        }
    }

    if !usable {
        return;
    }

    let Some(base) = flatten(manifest, &matrix.base, diags) else {
        return;
    };

    for combination in combinations(&matrix.axes) {
        let mut profile = base.clone();

        if let Some(output) = matrix.output.as_deref() {
            profile.output = Some(output.to_owned());
        }
        for (key, value) in &matrix.define {
            profile.define.insert(key.clone(), value.clone());
        }
        for (key, value) in &matrix.vars {
            profile.vars.insert(key.clone(), value.clone());
        }
        profile.define.sort_keys();
        profile.vars.sort_keys();

        if let Some(task) = task(name, &profile, &combination, root, overrides, diags) {
            tasks.push(task);
        }
    }
}

/// Axes sorted, values in declared order.
fn combinations(axes: &IndexMap<String, Vec<String>>) -> Vec<IndexMap<String, String>> {
    let mut result: Vec<IndexMap<String, String>> = vec![IndexMap::new()];

    for (axis, values) in axes {
        let mut next = Vec::with_capacity(result.len() * values.len());
        for partial in &result {
            for value in values {
                let mut extended = partial.clone();
                extended.insert(axis.clone(), value.clone());
                next.push(extended);
            }
        }
        result = next;
    }

    result
}

//! Collapsing an `extends` chain into one profile.

use crate::diag::Diag;
use crate::manifest::{Manifest, Profile};
use serde_json::Map;

use super::{CYCLIC_EXTENDS, UNKNOWN_BASE, known};

/// Nearer profiles win field by field. `vars` and `define` accumulate, and `darklua`
/// merges key by key so a profile can set `generator` without restating `bundle`.
pub fn flatten(manifest: &Manifest, name: &str, diags: &mut Vec<Diag>) -> Option<Profile> {
    let mut chain: Vec<&Profile> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut cursor = name;

    loop {
        // Also bounds the walk: a chain longer than the profile count must repeat.
        if seen.contains(&cursor) {
            seen.push(cursor);
            diags.push(
                Diag::deny(
                    CYCLIC_EXTENDS,
                    format!("profile `{name}` has a cyclic `extends` chain"),
                )
                .help(format!("the cycle is: {}", seen.join(", "))),
            );
            return None;
        }

        let Some(profile) = manifest.profiles.get(cursor) else {
            diags.push(
                Diag::deny(UNKNOWN_BASE, format!("profile `{cursor}` does not exist")).help(
                    format!("referenced by `{name}`, known: {}", known(manifest)),
                ),
            );
            return None;
        };

        seen.push(cursor);
        chain.push(profile);

        match profile.extends.as_deref() {
            Some(parent) => cursor = parent,
            None => break,
        }
    }

    let mut merged = Profile {
        vars: manifest.vars.clone(),
        ..Profile::default()
    };

    // `chain` runs child to ancestor, so fold from the ancestor end.
    for profile in chain.iter().rev() {
        merge(&mut merged, profile);
    }
    merged.extends = None;
    // Extending a template produces a real build.
    merged.is_abstract = false;

    Some(merged)
}

fn merge(base: &mut Profile, overlay: &Profile) {
    if overlay.entry.is_some() {
        base.entry.clone_from(&overlay.entry);
    }
    if overlay.output.is_some() {
        base.output.clone_from(&overlay.output);
    }
    if overlay.sources.is_some() {
        base.sources.clone_from(&overlay.sources);
    }
    if overlay.ignore.is_some() {
        base.ignore.clone_from(&overlay.ignore);
    }
    if overlay.header.is_some() {
        base.header.clone_from(&overlay.header);
    }
    if overlay.loaders.is_some() {
        base.loaders.clone_from(&overlay.loaders);
    }

    if let Some(overlay) = &overlay.darklua {
        let merged = base.darklua.get_or_insert_with(Map::new);
        for (key, value) in overlay {
            merged.insert(key.clone(), value.clone());
        }
    }

    for (key, value) in &overlay.vars {
        base.vars.insert(key.clone(), value.clone());
    }
    for (key, value) in &overlay.define {
        base.define.insert(key.clone(), value.clone());
    }
    base.vars.sort_keys();
    base.define.sort_keys();
}

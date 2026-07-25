//! Rebuilding on change.

use std::sync::mpsc;
use std::time::Duration;

use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::new_debouncer;

use crate::error::{Error, Result};
use crate::path::AbsPath;

use super::Scope;

/// Editors write a file in several operations, and a formatter on save produces more.
const SETTLE: Duration = Duration::from_millis(200);

/// Watches the same [`Scope`] the cache is keyed on.
pub fn run(scope: &Scope, manifest: &AbsPath, mut on_change: impl FnMut()) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(SETTLE, None, tx)
        .map_err(|e| Error::Watch("the filesystem".into(), e.to_string()))?;

    for root in scope.roots() {
        debouncer
            .watch(root.as_std(), RecursiveMode::Recursive)
            .map_err(|e| Error::Watch(root.to_string(), e.to_string()))?;
    }

    // A manifest reached with `-m` can sit outside every root. Its directory is watched
    // rather than the file, which editors replace on save.
    if !scope.roots().iter().any(|root| manifest.is_under(root))
        && let Some(directory) = manifest.parent()
    {
        debouncer
            .watch(directory.as_std(), RecursiveMode::NonRecursive)
            .map_err(|e| Error::Watch(directory.to_string(), e.to_string()))?;
    }

    // Watching is established first, so an edit made during the first build is queued.
    on_change();

    for batch in rx {
        // A failed batch means lost events.
        let Ok(events) = batch else { continue };

        if events
            .iter()
            .filter(|event| changed(&event.kind))
            .any(|event| {
                event
                    .paths
                    .iter()
                    .any(|path| path == manifest.as_std() || scope.contains(path))
            })
        {
            on_change();
        }
    }

    Ok(())
}

/// Reads are excluded: fingerprinting opens every input on each build.
fn changed(kind: &EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(modify) => !matches!(modify, notify::event::ModifyKind::Metadata(_)),
        _ => false,
    }
}

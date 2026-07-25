//! Rebuilding on change.
//!
//! The manifest is re-read on every cycle, so editing it takes effect without a
//! restart. That includes a change that makes it invalid, which is reported and then
//! waited on rather than being fatal.

use std::sync::mpsc;
use std::time::Duration;

use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::new_debouncer;

use crate::engine::Scope;
use crate::error::{Error, Result};
use crate::path::AbsPath;

/// How long to wait for a burst of filesystem events to settle.
///
/// Editors write a file in several operations, and a formatter on save produces more.
/// Rebuilding on the first one would build a half-written file.
const SETTLE: Duration = Duration::from_millis(200);

/// Watches everything `scope` covers, plus `manifest`, calling `on_change` after each
/// settled burst.
///
/// Runs until interrupted. `on_change` is called once before watching begins, so the
/// first build does not wait for an edit.
///
/// The watched set is the same [`Scope`] the cache is keyed on, so anything that would
/// invalidate a build wakes the watcher and nothing else does.
///
/// # Errors
///
/// When the platform watcher cannot be created, or a root cannot be watched.
pub fn run(scope: &Scope, manifest: &AbsPath, mut on_change: impl FnMut()) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(SETTLE, None, tx)
        .map_err(|e| Error::Watch("the filesystem".into(), e.to_string()))?;

    for root in scope.roots() {
        debouncer
            .watch(root.as_std(), RecursiveMode::Recursive)
            .map_err(|e| Error::Watch(root.to_string(), e.to_string()))?;
    }

    // A manifest reached with `-m` can sit outside every source root, and editing it
    // has to take effect. Watching its directory rather than the file survives the
    // rename-into-place that most editors save with.
    if !scope.roots().iter().any(|root| manifest.is_under(root))
        && let Some(directory) = manifest.parent()
    {
        debouncer
            .watch(directory.as_std(), RecursiveMode::NonRecursive)
            .map_err(|e| Error::Watch(directory.to_string(), e.to_string()))?;
    }

    // Watching is established first, so an edit made while the first build runs is
    // queued rather than lost.
    on_change();

    for batch in rx {
        // A failed batch means the watcher lost events, not that the project is
        // broken, so the next real edit still triggers a build.
        let Ok(events) = batch else { continue };

        if events
            .iter()
            .filter(|event| changes_content(&event.kind))
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

/// Whether an event represents a change to a file's contents or existence.
///
/// Reads are excluded deliberately. Fingerprinting opens every source file on each
/// build, and treating those opens as changes makes the watcher rebuild forever.
/// Metadata-only modifications are excluded for the same reason.
fn changes_content(kind: &EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(modify) => !matches!(modify, notify::event::ModifyKind::Metadata(_)),
        _ => false,
    }
}

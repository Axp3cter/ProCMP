//! Rebuilding on change.

use std::sync::mpsc;
use std::time::Duration;

use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::new_debouncer;

use super::Engine;
use crate::cli::render;
use crate::plan::Plan;
use crate::report::{Code, Diagnostic, Exit};
use crate::vfs::AbsPath;

/// Editors write a file in several operations, and a formatter on save produces more.
const SETTLE: Duration = Duration::from_millis(200);

/// Watches the manifest's directory and every extra root the selection declares.
///
/// The plan is resolved once. Re-reading the manifest each cycle would mean a half-saved
/// manifest producing a cascade of parse errors, and the useful signal — that sources
/// changed — arrives far more often than the manifest does.
pub fn run(
    root: &AbsPath,
    cache: &AbsPath,
    plan: &Plan,
    selection: &Plan,
    emit: bool,
) -> Result<Exit, Diagnostic> {
    let (sender, receiver) = mpsc::channel();
    let mut debouncer = new_debouncer(SETTLE, None, sender).map_err(|error| {
        Diagnostic::new(Code::Unreadable, "could not watch the filesystem").help(error.to_string())
    })?;

    let mut roots = vec![root.clone()];
    roots.extend(
        selection
            .tasks
            .iter()
            .flat_map(|task| &task.sources)
            .filter_map(|source| root.join(source.as_str()).ok()),
    );
    roots.sort();
    roots.dedup();

    for directory in &roots {
        debouncer
            .watch(directory.as_std(), RecursiveMode::Recursive)
            .map_err(|error| {
                Diagnostic::new(Code::Unreadable, format!("could not watch `{directory}`"))
                    .help(error.to_string())
            })?;
    }

    // What a build writes itself. Without this the first cycle's own records and artifacts
    // arrive as events and set off a cascade of cycles that find nothing to do.
    let mut ours = vec![cache.clone()];
    ours.extend(
        plan.tasks
            .iter()
            .filter_map(|task| root.join(task.output.as_str()).ok()),
    );

    let engine = Engine::new(root.clone(), cache.clone(), true);

    // Watching is established first, so an edit made during the first build is queued
    // rather than lost.
    render::build(&engine.run(plan, selection), emit, !emit, false);

    for batch in receiver {
        // A failed batch means lost events, and a rebuild is cheaper than a wrong answer.
        let rebuild = batch.map_or(true, |events| {
            events
                .iter()
                .filter(|event| changed(event.kind))
                .flat_map(|event| &event.paths)
                .any(|path| ours_not(path, &ours))
        });

        if rebuild {
            render::build(&engine.run(plan, selection), emit, !emit, false);
        }
    }

    Ok(Exit::Success)
}

/// Whether a changed path is something the project owns rather than something a build
/// just produced.
fn ours_not(path: &std::path::Path, ours: &[AbsPath]) -> bool {
    let Some(path) = path.to_str().and_then(|text| AbsPath::new(text).ok()) else {
        return false;
    };
    if path.as_str().contains("/.git/")
        || path
            .file_name()
            .is_some_and(|name| name.starts_with(crate::vfs::TEMPORARY))
    {
        return false;
    }

    !ours.iter().any(|written| path.is_under(written))
}

/// Reads are excluded: fingerprinting opens every input on each build.
fn changed(kind: EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(modify) => !matches!(modify, notify::event::ModifyKind::Metadata(_)),
        _ => false,
    }
}

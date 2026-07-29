//! Everything written to a stream.
//!
//! The only module that prints, which the crate's `print_stdout` and `print_stderr` lints
//! make a compile error to forget. `println!` is avoided too: it panics on a closed pipe,
//! which `pcmp plan | head` produces routinely.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the one module that writes to a stream"
)]

use std::fmt::Write as _;
use std::io::Write as _;

use crate::build::{Report, Status};
use crate::plan::{Plan, Task};
use crate::report::{self, Diagnostic, Severity};
use crate::vfs::RelPath;

pub fn line(text: impl std::fmt::Display) {
    let _ = writeln!(std::io::stdout(), "{text}");
}

pub fn problem(text: impl std::fmt::Display) {
    let _ = writeln!(std::io::stderr(), "{text}");
}

fn json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(rendered) => line(rendered),
        // On stderr, so stdout stays parseable.
        Err(error) => problem(format!("error: could not encode output: {error}")),
    }
}

/// Measured in characters, not bytes.
fn pad(text: &str, to: usize) -> String {
    let width = text.chars().count();
    if width >= to {
        text.to_owned()
    } else {
        format!("{text}{}", " ".repeat(to - width))
    }
}

fn widest<'a>(items: impl Iterator<Item = &'a str>) -> usize {
    items.map(|text| text.chars().count()).max().unwrap_or(0)
}

pub fn plan(plan: &Plan, emit: bool) {
    if emit {
        return json(plan);
    }
    if plan.is_empty() {
        return line("no tasks");
    }

    line(format!(
        "{} task(s), plan {}\n",
        plan.len(),
        plan.digest().short()
    ));

    let rows: Vec<(&str, &str, String)> = plan
        .tasks
        .iter()
        .map(|task| {
            let rules = task.config.rules.as_ref().map_or_else(
                || "darklua defaults".to_owned(),
                |r| format!("{} rules", r.len()),
            );
            (task.id.as_str(), task.output.as_str(), rules)
        })
        .collect();

    let id = widest(rows.iter().map(|(id, _, _)| *id));
    let output = widest(rows.iter().map(|(_, output, _)| *output));

    for (task, artifact, rules) in &rows {
        line(format!(
            "  {}  {}  {rules}",
            pad(task, id),
            pad(artifact, output)
        ));
    }
}

/// What `explain <TASK>` used to print, now reached by naming a task to `plan`.
pub fn task(task: &Task, emit: bool) {
    if emit {
        return json(&serde_json::json!({
            "task": task,
            "digest": task.digest().to_string(),
            "darklua": task.config.json(),
        }));
    }

    line(format!("task     {}", task.id));
    line(format!("entry    {}", task.entry));
    line(format!("output   {}", task.output));
    line(format!("digest   {}\n", task.digest().short()));

    table(
        "vars",
        task.vars.iter().map(|(k, v)| (k.to_string(), v.text())),
    );
    line("");
    table(
        "defines",
        task.defines
            .iter()
            .map(|(k, v)| (k.to_string(), v.tagged())),
    );

    line("\ndarklua configuration");
    let config = serde_json::to_string_pretty(&task.config.json()).unwrap_or_default();
    for text in config.lines() {
        line(format!("  {text}"));
    }
}

fn table(label: &str, rows: impl Iterator<Item = (String, String)>) {
    let rows: Vec<_> = rows.collect();
    if rows.is_empty() {
        return line(format!("{label}  <none>"));
    }

    line(label);
    let width = widest(rows.iter().map(|(key, _)| key.as_str()));
    for (key, value) in &rows {
        line(format!("  {}  {value}", pad(key, width)));
    }
}

pub fn build(report: &Report, emit: bool, timings: bool, why: bool) {
    if emit {
        return json(report);
    }

    let id = widest(report.tasks.iter().map(|task| task.task.as_str()));
    let output = widest(report.tasks.iter().map(|task| task.output.as_str()));

    for task in &report.tasks {
        let label = match task.status {
            Status::Built => "built ",
            Status::Cached => "cached",
            Status::Failed => "FAILED",
        };

        let mut row = format!(
            "  {label}  {}  {}",
            pad(task.task.as_str(), id),
            pad(task.output.as_str(), output)
        );
        if timings {
            let _ = write!(row, "  ({} ms)", task.millis);
        }
        if why && let Some(reason) = task.why {
            let _ = write!(row, "  — {}", reason.describe());
        }
        line(row);

        for diagnostic in &task.diagnostics {
            for text in rendered(diagnostic).lines() {
                line(format!("          {text}"));
            }
        }
    }

    let (built, cached, failed) = report.counts();
    line(format!("\n{built} built, {cached} cached, {failed} failed"));
}

/// Findings that are the command's answer, as `check`'s are.
pub fn diagnostics(diagnostics: &[Diagnostic], emit: bool) {
    report_to(diagnostics, emit, line);
}

/// Findings that mean the command failed.
///
/// On stderr, so that `pcmp build > artifacts.json` still says why it did not work. With
/// `--json` both go to stdout, because that is where machine output lives and the exit
/// code already says which happened.
pub fn failures(diagnostics: &[Diagnostic], emit: bool) {
    if emit {
        json(&diagnostics);
    } else {
        report_to(diagnostics, false, problem);
    }
}

fn report_to(diagnostics: &[Diagnostic], emit: bool, out: fn(String)) {
    if emit {
        return json(&diagnostics);
    }
    if diagnostics.is_empty() {
        return out("no findings".to_owned());
    }

    for diagnostic in diagnostics {
        out(rendered(diagnostic));
        out(String::new());
    }

    let (errors, warnings) = report::tally(diagnostics);
    out(format!("{errors} error(s), {warnings} warning(s)"));
}

/// One diagnostic, in the shape `error[missing-output]: …` — code first, so the reader
/// knows what to pass to `pcmp explain`.
pub fn rendered(diagnostic: &Diagnostic) -> String {
    let marker = match diagnostic.severity() {
        Severity::Error => "error",
        Severity::Warning => "warn",
    };

    let mut out = format!(
        "{marker}[{}]: {}",
        diagnostic.code.slug(),
        diagnostic.message
    );

    if let Some(at) = &diagnostic.at {
        let _ = write!(out, "\n  at {at}");
    }
    if let Some(source) = &diagnostic.source {
        let _ = write!(out, "\n  {source}");
    }
    for help in diagnostic.help.iter().flat_map(|help| help.lines()) {
        let _ = write!(out, "\n  help: {help}");
    }

    out
}

pub fn created(paths: &[RelPath]) {
    for path in paths {
        line(format!("created  {path}"));
    }
    line("");
    line("next     pcmp plan");
}

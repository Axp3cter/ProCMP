//! Everything written to stdout.

use std::io::Write;

use procmp::build::{Outcome, Report, config_json};
use procmp::diag::{self, Diag, Severity};
use procmp::{AbsPath, Graph, Task};

/// `println!` panics when the reader goes away, so `pcmp plan | head` would abort with
/// a backtrace. A closed pipe is a normal end to output.
pub fn outln(text: impl std::fmt::Display) {
    let _ = writeln!(std::io::stdout(), "{text}");
}

fn emit<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(rendered) => outln(rendered),
        // On stderr, so stdout stays parseable.
        Err(error) => eprintln!("error: could not encode output: {error}"),
    }
}

/// Measured in characters, so non-ASCII names keep their alignment.
fn pad(text: &str, to: usize) -> String {
    let length = text.chars().count();
    match length >= to {
        true => text.to_owned(),
        false => format!("{text}{}", " ".repeat(to - length)),
    }
}

fn widest<'a>(items: impl Iterator<Item = &'a str>) -> usize {
    items.map(|text| text.chars().count()).max().unwrap_or(0)
}

pub fn plan(graph: &Graph, root: &AbsPath, json: bool) {
    if json {
        return emit(graph);
    }
    if graph.is_empty() {
        return outln("no tasks");
    }

    let width = widest(graph.tasks.iter().map(|t| t.id.as_str()));
    outln(format!(
        "{} task(s), plan {}\n",
        graph.len(),
        graph.digest().short()
    ));

    for task in &graph.tasks {
        let rules = task.rules.as_ref().map_or_else(
            || "darklua defaults".to_owned(),
            |r| format!("{} rules", r.len()),
        );
        outln(format!(
            "  {}  {}  {rules}",
            pad(&task.id, width),
            task.output.relative_to(root)
        ));
    }
}

pub fn explain(task: &Task, root: &AbsPath, json: bool) {
    if json {
        return emit(&serde_json::json!({
            "task": task,
            "darklua": config_json(task),
        }));
    }

    outln(format!("task     {}", task.id));
    outln(format!("entry    {}", task.entry.relative_to(root)));
    outln(format!("output   {}", task.output.relative_to(root)));
    outln(format!("digest   {}\n", task.digest().short()));

    table(
        "vars",
        task.vars.iter().map(|(k, v)| (k.clone(), v.clone())),
    );
    outln("");
    table(
        "defines",
        task.defines.iter().map(|(k, v)| (k.clone(), v.tagged())),
    );

    outln("\ndarklua configuration");
    let config = serde_json::to_string_pretty(&config_json(task)).unwrap_or_default();
    for line in config.lines() {
        outln(format!("  {line}"));
    }
}

fn table(label: &str, rows: impl Iterator<Item = (String, String)>) {
    let rows: Vec<_> = rows.collect();

    if rows.is_empty() {
        return outln(format!("{label}  <none>"));
    }

    outln(label);
    let width = widest(rows.iter().map(|(key, _)| key.as_str()));
    for (key, value) in &rows {
        outln(format!("  {}  {value}", pad(key, width)));
    }
}

pub fn build(report: &Report, json: bool) {
    if json {
        return emit(report);
    }

    let width = widest(report.tasks.iter().map(|t| t.task.as_str()));

    for task in &report.tasks {
        let label = match &task.outcome {
            Outcome::Built { .. } => "built ",
            Outcome::Cached { .. } => "cached",
            Outcome::Failed { .. } => "FAILED",
        };
        outln(format!(
            "  {label}  {}  {}  ({} ms)",
            pad(&task.task, width),
            task.output,
            task.millis
        ));

        if let Outcome::Failed { reason } = &task.outcome {
            for line in reason.lines() {
                outln(format!("          {line}"));
            }
        }
    }

    let (built, cached, failed) = report.counts();
    outln(format!("\n{built} built, {cached} cached, {failed} failed"));
}

pub fn diags(diags: &[Diag], json: bool) {
    if json {
        return emit(&diags);
    }
    if diags.is_empty() {
        return outln("no findings");
    }

    for diag in diags {
        let marker = match diag.severity {
            Severity::Deny => "error",
            Severity::Warn => "warn ",
        };
        outln(format!("{marker}  {}: {}", diag.code, diag.message));

        for line in diag.help.iter().flat_map(|help| help.lines()) {
            outln(format!("       help: {line}"));
        }
        outln("");
    }

    let (errors, warnings) = diag::tally(diags);
    outln(format!("{errors} error(s), {warnings} warning(s)"));
}

pub fn verify(differing: &[&str], total: usize, json: bool) {
    if json {
        return emit(&serde_json::json!({
            "reproducible": differing.is_empty(),
            "tasks": total,
            "differing": differing,
        }));
    }

    if differing.is_empty() {
        return outln(format!(
            "reproducible: {total} task(s) byte-identical across two builds"
        ));
    }

    outln(format!(
        "NOT reproducible: {} of {total} differ",
        differing.len()
    ));
    for id in differing {
        outln(format!("  {id}"));
    }
}

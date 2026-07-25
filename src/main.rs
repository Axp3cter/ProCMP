//! The `pcmp` command line: parse, call into [`procmp`], render, exit.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

mod pick;

use clap::{Parser, Subcommand, ValueEnum};
use procmp::diag::{self, Diag, Severity};
use procmp::error::{Error, ExitCode, Result};
use procmp::{
    AbsPath, Engine, Graph, Outcome, Overrides, engine, init, lint, load, plan, schema, watch,
};

#[derive(Debug, Parser)]
#[command(
    name = "pcmp",
    version,
    // Built at compile time from the same lockfile `build.rs` read, so it cannot
    // disagree with the version mixed into cache keys.
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\ndarklua ",
        env!("DARKLUA_VERSION")
    ),
    about = "Multi-target build composition for Luau projects"
)]
struct Cli {
    /// Path to the manifest. Discovered from the working directory upwards when
    /// omitted.
    #[arg(long, short, global = true)]
    manifest: Option<String>,

    /// Where build state is kept. Defaults to `.pcmp/` beside the manifest.
    #[arg(long, global = true, value_name = "PATH")]
    cache_dir: Option<String>,

    /// Supply a value to `pcmp.env`, as `KEY=VALUE`. Repeatable.
    ///
    /// Read by the manifest but never exported, so it cannot reach anything ProCMP
    /// runs.
    #[arg(long = "env", short = 'e', global = true, value_name = "KEY=VALUE")]
    env: Vec<String>,

    /// Set a token, as `KEY=VALUE`. Repeatable. Beats the manifest.
    #[arg(long = "var", global = true, value_name = "KEY=VALUE")]
    var: Vec<String>,

    /// Set a constant, as `KEY=VALUE`. Repeatable. Beats the manifest.
    ///
    /// `true`, `false` and numbers are read as such. Anything else stays a string.
    #[arg(long = "define", short = 'D', global = true, value_name = "KEY=VALUE")]
    define: Vec<String>,

    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolve the manifest and print the plan without running it.
    Plan,

    /// Build every task, or those matching a pattern.
    Build {
        /// Task or profile names. `*` is accepted. All when omitted.
        tasks: Vec<String>,
        /// Choose from a list instead. Needs a terminal.
        #[arg(long)]
        pick: bool,
        /// Ignore cached state.
        #[arg(long)]
        no_cache: bool,
    },

    /// Lint the manifest and the resolved plan.
    Check {
        /// Fail on warnings too.
        #[arg(long)]
        strict: bool,
    },

    /// Print the darklua configuration a task compiles down to.
    Explain {
        /// Task name. Required unless `--pick` is given.
        task: Option<String>,
        /// Choose from a list instead. Needs a terminal.
        #[arg(long)]
        pick: bool,
    },

    /// Emit the manifest schema.
    Schema {
        #[arg(long, value_enum, default_value_t = Kind::Json)]
        format: Kind,
    },

    /// Build twice and prove the output is byte-identical.
    Verify,

    /// Rebuild whenever an input or the manifest changes.
    Watch {
        /// Task or profile names. `*` is accepted. All when omitted.
        tasks: Vec<String>,
        /// Choose from a list instead. Needs a terminal.
        #[arg(long)]
        pick: bool,
    },

    /// Write a starter manifest and its schema.
    Init {
        /// Project name. Defaults to the directory name.
        #[arg(long)]
        name: Option<String>,
        /// Entry point. Detected from common locations when omitted.
        #[arg(long)]
        entry: Option<String>,
        /// Manifest format to scaffold.
        #[arg(long, value_enum, default_value_t = init::Format::Json5)]
        format: init::Format,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Kind {
    Json,
    Luau,
}

/// Writes a line, tolerating a closed pipe.
///
/// `println!` panics when the reader goes away, so `pcmp plan | head` would abort with
/// a backtrace. A closed pipe is a normal end to output, so the error is discarded.
macro_rules! outln {
    () => { { use std::io::Write as _; let _ = writeln!(std::io::stdout()); } };
    ($($arg:tt)*) => { {
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    } };
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    let code = match run(&cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::Config
        }
    };

    std::process::ExitCode::from(code as u8)
}

fn run(cli: &Cli) -> Result<ExitCode> {
    let cwd = AbsPath::cwd()?;

    // Handled before a manifest is looked for: one needs no project, the other
    // exists precisely because there is not one yet.
    match &cli.command {
        Command::Schema { format } => {
            outln!(
                "{}",
                match format {
                    Kind::Json => schema::json(),
                    Kind::Luau => schema::luau(),
                }
            );
            return Ok(ExitCode::Success);
        }
        Command::Init {
            name,
            entry,
            format,
        } => {
            let name = name
                .clone()
                .unwrap_or_else(|| cwd.file_name().unwrap_or("project").to_owned());
            let created = init::run(&cwd, &name, entry.as_deref(), *format)?;

            outln!("created  {}", created.manifest.relative_to(&cwd));
            outln!("created  {}", created.definitions.relative_to(&cwd));
            outln!();
            outln!("next     pcmp plan");
            return Ok(ExitCode::Success);
        }
        _ => {}
    }

    let env = load::Env::parse(&cli.env)?;
    let overrides = Overrides::parse(&cli.define, &cli.var)?;

    let path = match cli.manifest.as_deref() {
        Some(path) => cwd.join(path)?,
        None => load::discover(&cwd)?,
    };

    let loaded = load::load(&path, &env)?;
    let cache = match cli.cache_dir.as_deref() {
        Some(dir) => cwd.join(dir)?,
        None => loaded.root.join(engine::CACHE_DIR)?,
    };
    let resolution = plan::resolve(&loaded.manifest, &loaded.root, &overrides);

    // A denied resolution has no graph, so the findings are the useful output.
    let Some(graph) = resolution.graph else {
        print_diags(&resolution.diags, cli.json);
        return Err(Error::Unresolved(path.relative_to(&cwd)));
    };

    match &cli.command {
        Command::Plan => {
            print_plan(&graph, &loaded.root, cli.json);
            Ok(ExitCode::Success)
        }

        Command::Check { strict } => {
            let mut diags = resolution.diags;
            diags.extend(lint::run(&loaded.manifest, &graph));
            diag::sort(&mut diags);
            print_diags(&diags, cli.json);

            Ok(match diag::worst(&diags) {
                Some(Severity::Deny) => ExitCode::Lint,
                Some(Severity::Warn) if *strict => ExitCode::Lint,
                _ => ExitCode::Success,
            })
        }

        Command::Explain { task, pick } => {
            let task = match (task, pick) {
                (_, true) => match pick::tasks(&graph, &loaded.root, "explain", pick::Mode::One)? {
                    Some(indices) => &graph.tasks[indices[0]],
                    None => return Ok(ExitCode::Success),
                },
                (Some(name), false) => graph
                    .get(name)
                    .ok_or_else(|| Error::NoSuchTask(name.clone(), graph.known()))?,
                (None, false) => return Err(Error::NoTaskGiven(graph.known())),
            };
            print_explain(task, &loaded.root, cli.json);
            Ok(ExitCode::Success)
        }

        Command::Build {
            tasks,
            pick,
            no_cache,
        } => {
            let Some(selected) = choose(&graph, tasks, *pick, &loaded.root, "build")? else {
                return Ok(ExitCode::Success);
            };
            let report = Engine::new(loaded.root.clone(), cache)
                .cached(!no_cache)
                .run(&selected)?;

            print_build(&report, cli.json);
            Ok(if report.succeeded() {
                ExitCode::Success
            } else {
                ExitCode::Build
            })
        }

        Command::Verify => {
            let engine = Engine::new(loaded.root.clone(), cache).cached(false);

            // A build that failed has no artifact to compare. Without this the
            // comparison reports a missing file instead of the failure that caused it.
            let Some(first) = pass(&engine, &graph, cli.json)? else {
                return Ok(ExitCode::Build);
            };
            let Some(second) = pass(&engine, &graph, cli.json)? else {
                return Ok(ExitCode::Build);
            };

            let differing: Vec<&str> = first
                .iter()
                .zip(&second)
                .filter(|((_, a), (_, b))| a != b)
                .map(|((id, _), _)| *id)
                .collect();

            let result = Reproducibility {
                differing,
                total: graph.len(),
            };
            let reproducible = result.differing.is_empty();

            print_verify(&result, cli.json);
            Ok(if reproducible {
                ExitCode::Success
            } else {
                ExitCode::Build
            })
        }

        // Watching re-reads the manifest each cycle, so a manifest edit takes effect
        // without a restart. That includes one that breaks it, which is reported and
        // waited on rather than ending the session.
        Command::Watch { tasks, pick } => {
            let Some(selected) = choose(&graph, tasks, *pick, &loaded.root, "watch")? else {
                return Ok(ExitCode::Success);
            };

            // Resolved once here so a picked set survives the manifest being re-read.
            let names: Vec<String> = selected.tasks.iter().map(|t| t.id.clone()).collect();
            let scope = engine::Scope::of(&selected, &cache)?;
            let json = cli.json;

            watch::run(&scope, &path, || {
                match rebuild(&path, &cache, &names, json, &env, &overrides) {
                    Ok(()) => {}
                    Err(error) => eprintln!("error: {error}"),
                }
            })?;

            Ok(ExitCode::Success)
        }

        Command::Schema { .. } | Command::Init { .. } => unreachable!("handled above"),
    }
}

/// One watch cycle: re-read, re-resolve, rebuild.
fn rebuild(
    manifest: &AbsPath,
    cache: &AbsPath,
    selectors: &[String],
    json: bool,
    env: &load::Env,
    overrides: &Overrides,
) -> Result<()> {
    let loaded = load::load(manifest, env)?;
    let resolution = plan::resolve(&loaded.manifest, &loaded.root, overrides);

    let Some(graph) = resolution.graph else {
        print_diags(&resolution.diags, json);
        return Ok(());
    };

    let report =
        Engine::new(loaded.root.clone(), cache.clone()).run(&select(&graph, selectors)?)?;
    print_build(&report, json);
    Ok(())
}

/// One `verify` pass: build, then fingerprint what was written.
///
/// [`None`] when the build itself failed, which this reports before returning.
fn pass<'g>(
    engine: &Engine,
    graph: &'g Graph,
    json: bool,
) -> Result<Option<Vec<(&'g str, String)>>> {
    let report = engine.run(graph)?;

    if !report.succeeded() {
        print_build(&report, json);
        return Ok(None);
    }

    fingerprints(graph).map(Some)
}

/// Hashes what each task produced. A directory output folds in every file beneath it,
/// so `verify` proves a whole processed tree, not just a bundle.
fn fingerprints(graph: &Graph) -> Result<Vec<(&str, String)>> {
    graph
        .tasks
        .iter()
        .map(|task| {
            let mut hasher = procmp::Hasher::new();

            for artifact in engine::artifacts(&task.output)? {
                let bytes = std::fs::read(artifact.as_std())
                    .map_err(|e| Error::Read(artifact.to_string(), e.to_string()))?;
                hasher.field(
                    &artifact.relative_to(&task.output),
                    procmp::digest::of(bytes).bytes(),
                );
            }

            Ok((task.id.as_str(), hasher.finish().hex()))
        })
        .collect()
}

/// Applies `--pick` when given, otherwise the names on the command line.
///
/// [`None`] when the picker was cancelled, which is an ordinary exit rather than an
/// error: the user asked to choose and chose nothing.
fn choose(
    graph: &Graph,
    selectors: &[String],
    pick: bool,
    root: &AbsPath,
    title: &str,
) -> Result<Option<Graph>> {
    if !pick {
        return select(graph, selectors).map(Some);
    }

    let Some(indices) = pick::tasks(graph, root, title, pick::Mode::Many)? else {
        return Ok(None);
    };

    Ok(Some(Graph {
        tasks: indices.iter().map(|at| graph.tasks[*at].clone()).collect(),
    }))
}

/// Selects tasks by identifier or profile name, with `*` standing for any run of
/// characters.
///
/// Only `*`, because a matrix identifier contains `[`, `]` and `=`, which every glob
/// dialect reads as syntax. `pcmp build 'dist[target=roblox]'` would stop meaning what
/// it says. `*target=roblox*` selects across a matrix instead.
///
/// An empty selection is an error rather than a no-op, so a mistyped name in CI cannot
/// pass as a successful build.
fn select(graph: &Graph, selectors: &[String]) -> Result<Graph> {
    if selectors.is_empty() {
        return Ok(graph.clone());
    }

    let tasks: Vec<_> = graph
        .tasks
        .iter()
        .filter(|task| {
            selectors
                .iter()
                .any(|s| matches(s, &task.id) || matches(s, &task.profile))
        })
        .cloned()
        .collect();

    if tasks.is_empty() {
        return Err(Error::NoSuchTask(selectors.join(", "), graph.known()));
    }

    Ok(Graph { tasks })
}

/// Whether `text` matches `pattern`, where `*` matches any run of characters.
///
/// The pattern splits into literals that must appear in order: the first anchored to
/// the start, the last to the end, the rest anywhere between. A pattern with no `*` is
/// one literal anchored at both ends, which is an exact match.
fn matches(pattern: &str, text: &str) -> bool {
    let literals: Vec<&str> = pattern.split('*').collect();

    let (Some(first), Some(last)) = (literals.first(), literals.last()) else {
        return false;
    };

    let Some(rest) = text.strip_prefix(first) else {
        return false;
    };
    if literals.len() == 1 {
        return rest.is_empty();
    }

    // The tail is claimed before the middles are searched, so neither can consume the
    // characters the other needs.
    let Some(mut rest) = rest.strip_suffix(last) else {
        return false;
    };

    for middle in &literals[1..literals.len() - 1] {
        let Some(at) = rest.find(middle) else {
            return false;
        };
        rest = &rest[at + middle.len()..];
    }

    true
}

/// What two builds of the same graph agreed on.
struct Reproducibility<'g> {
    differing: Vec<&'g str>,
    total: usize,
}

// ── rendering ───────────────────────────────────────────────────────────────────

/// Measured in characters, not bytes, so non-ASCII names keep their alignment.
fn pad(text: &str, to: usize) -> String {
    let length = text.chars().count();
    if length >= to {
        return text.to_owned();
    }
    format!("{text}{}", " ".repeat(to - length))
}

fn emit<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(rendered) => outln!("{rendered}"),
        // Reported on stderr so stdout stays parseable by whatever consumes it.
        Err(error) => eprintln!("error: could not encode output: {error}"),
    }
}

fn print_plan(graph: &Graph, root: &AbsPath, json: bool) {
    if json {
        return emit(graph);
    }
    if graph.is_empty() {
        return outln!("no tasks");
    }

    let width = graph
        .tasks
        .iter()
        .map(|t| t.id.chars().count())
        .max()
        .unwrap_or(0);
    outln!("{} task(s), plan {}\n", graph.len(), graph.digest().short());

    for task in &graph.tasks {
        outln!(
            "  {}  {}  {}",
            pad(&task.id, width),
            task.output.relative_to(root),
            task.rules.as_ref().map_or_else(
                || "darklua defaults".to_owned(),
                |r| format!("{} rules", r.len())
            )
        );
    }
}

fn print_explain(task: &plan::Task, root: &AbsPath, json: bool) {
    if json {
        return emit(&serde_json::json!({
            "task": task,
            "darklua": engine::config_json(task),
        }));
    }

    outln!("task     {}", task.id);
    outln!("entry    {}", task.entry.relative_to(root));
    outln!("output   {}", task.output.relative_to(root));
    outln!("digest   {}\n", task.digest().short());

    table(
        "vars",
        task.vars.iter().map(|(k, v)| (k.clone(), v.clone())),
    );
    outln!();
    table(
        "defines",
        task.defines.iter().map(|(k, v)| (k.clone(), v.tagged())),
    );

    outln!("\ndarklua configuration");
    let config = serde_json::to_string_pretty(&engine::config_json(task)).unwrap_or_default();
    for line in config.lines() {
        outln!("  {line}");
    }
}

/// Prints a labelled key-value block, or `<none>` when there is nothing in it.
fn table(label: &str, rows: impl Iterator<Item = (String, String)>) {
    let rows: Vec<_> = rows.collect();

    if rows.is_empty() {
        return outln!("{label}  <none>");
    }

    outln!("{label}");
    let width = rows
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    for (key, value) in &rows {
        outln!("  {}  {value}", pad(key, width));
    }
}

fn print_build(report: &engine::Report, json: bool) {
    if json {
        return emit(report);
    }

    let width = report
        .tasks
        .iter()
        .map(|t| t.task.chars().count())
        .max()
        .unwrap_or(0);

    for task in &report.tasks {
        let label = match &task.outcome {
            Outcome::Built { .. } => "built ",
            Outcome::Cached { .. } => "cached",
            Outcome::Failed { .. } => "FAILED",
        };
        outln!(
            "  {label}  {}  {}  ({} ms)",
            pad(&task.task, width),
            task.output,
            task.millis
        );

        if let Outcome::Failed { reason } = &task.outcome {
            for line in reason.lines() {
                outln!("          {line}");
            }
        }
    }

    let (built, cached, failed) = report.counts();
    outln!("\n{built} built, {cached} cached, {failed} failed");
}

/// Renders findings from `pcmp check`, or from a manifest that failed to resolve.
fn print_diags(diags: &[Diag], json: bool) {
    if json {
        return emit(&diags);
    }
    if diags.is_empty() {
        return outln!("no findings");
    }

    for d in diags {
        let marker = match d.severity {
            Severity::Deny => "error",
            Severity::Warn => "warn ",
        };
        outln!("{marker}  {}: {}", d.code, d.message);
        if let Some(help) = &d.help {
            for line in help.lines() {
                outln!("       help: {line}");
            }
        }
        outln!();
    }

    let (errors, warnings) = diag::tally(diags);
    outln!("{errors} error(s), {warnings} warning(s)");
}

fn print_verify(result: &Reproducibility<'_>, json: bool) {
    let Reproducibility { differing, total } = result;

    if json {
        return emit(&serde_json::json!({
            "reproducible": differing.is_empty(),
            "tasks": total,
            "differing": differing,
        }));
    }

    if differing.is_empty() {
        outln!("reproducible: {total} task(s) byte-identical across two builds");
        return;
    }

    outln!("NOT reproducible: {} of {total} differ", differing.len());
    for id in differing {
        outln!("  {id}");
    }
}

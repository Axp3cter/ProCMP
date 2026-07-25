//! Argument parsing and dispatch.

mod menu;
mod render;

use clap::{Parser, Subcommand, ValueEnum};
use procmp::diag::{self, Severity};
use procmp::error::{Error, ExitCode, Result};
use procmp::{
    AbsPath, Engine, Graph, Overrides, Scope, build, check, init, manifest, plan, schema,
};

use render::outln;

#[derive(Debug, Parser)]
#[command(
    name = "pcmp",
    version,
    long_version = concat!(env!("CARGO_PKG_VERSION"), "\ndarklua ", env!("DARKLUA_VERSION")),
    about = "Multi-target build composition for Luau projects"
)]
pub struct Cli {
    /// Manifest to use. Discovered from the working directory upwards otherwise.
    #[arg(long, short, global = true)]
    manifest: Option<String>,

    /// Where build state is kept. Defaults to `.pcmp/` beside the manifest.
    #[arg(long, global = true, value_name = "PATH")]
    cache_dir: Option<String>,

    /// A value for `pcmp.env`, ahead of the process environment. Repeatable.
    #[arg(long = "env", short = 'e', global = true, value_name = "KEY=VALUE")]
    env: Vec<String>,

    /// Set a token. Repeatable. Beats the manifest.
    #[arg(long = "var", global = true, value_name = "KEY=VALUE")]
    var: Vec<String>,

    /// Set a constant. Repeatable. Beats the manifest.
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

    /// Build every task, or a selection.
    Build {
        /// Task or profile names. `*` is accepted. All when omitted.
        tasks: Vec<String>,
        /// Choose from a menu instead. Needs a terminal.
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

    /// Print the darklua configuration a task compiles to.
    Explain {
        /// Required unless `--pick` is given.
        task: Option<String>,
        #[arg(long)]
        pick: bool,
    },

    /// Emit the manifest schema.
    Schema {
        #[arg(long, value_enum, default_value_t = Shape::Json)]
        format: Shape,
    },

    /// Build twice and prove the output is byte-identical.
    Verify,

    /// Rebuild whenever an input or the manifest changes.
    Watch {
        /// Task or profile names. `*` is accepted. All when omitted.
        tasks: Vec<String>,
        #[arg(long)]
        pick: bool,
    },

    /// Write a starter manifest and its schema.
    Init {
        /// Defaults to the directory name.
        #[arg(long)]
        name: Option<String>,
        /// Detected from common locations when omitted.
        #[arg(long)]
        entry: Option<String>,
        #[arg(long, value_enum, default_value_t = init::Format::Json5)]
        format: init::Format,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Shape {
    Json,
    Luau,
}

pub fn run(cli: &Cli) -> Result<ExitCode> {
    let cwd = AbsPath::cwd()?;

    // Neither needs a manifest.
    match &cli.command {
        Command::Schema { format } => {
            outln(match format {
                Shape::Json => schema::json(),
                Shape::Luau => schema::luau(),
            });
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

            outln(format!("created  {}", created.manifest.relative_to(&cwd)));
            outln(format!(
                "created  {}",
                created.definitions.relative_to(&cwd)
            ));
            outln("");
            outln("next     pcmp plan");
            return Ok(ExitCode::Success);
        }
        _ => {}
    }

    let env = manifest::Env::parse(&cli.env)?;
    let overrides = Overrides::parse(&cli.define, &cli.var)?;

    let path = match cli.manifest.as_deref() {
        Some(path) => cwd.join(path)?,
        None => manifest::discover(&cwd)?,
    };

    let loaded = manifest::load(&path, &env)?;
    let cache = match cli.cache_dir.as_deref() {
        Some(dir) => cwd.join(dir)?,
        None => loaded.root.join(build::CACHE_DIR)?,
    };
    let resolution = plan::resolve(&loaded.manifest, &loaded.root, &overrides);

    let Some(graph) = resolution.graph else {
        render::diags(&resolution.diags, cli.json);
        return Err(Error::Unresolved(path.relative_to(&cwd)));
    };

    match &cli.command {
        Command::Plan => {
            render::plan(&graph, &loaded.root, cli.json);
            Ok(ExitCode::Success)
        }

        Command::Check { strict } => {
            let mut diags = resolution.diags;
            diags.extend(check::run(&loaded.manifest, &graph));
            diag::sort(&mut diags);
            render::diags(&diags, cli.json);

            Ok(match diag::worst(&diags) {
                Some(Severity::Deny) => ExitCode::Lint,
                Some(Severity::Warn) if *strict => ExitCode::Lint,
                _ => ExitCode::Success,
            })
        }

        Command::Explain { task, pick } => {
            let task = match (task, pick) {
                (_, true) => match menu::tasks(&graph, &loaded.root, "explain", menu::Mode::One)? {
                    Some(indices) => &graph.tasks[indices[0]],
                    None => return Ok(ExitCode::Success),
                },
                (Some(name), false) => graph
                    .get(name)
                    .ok_or_else(|| Error::NoSuchTask(name.clone(), graph.known()))?,
                (None, false) => return Err(Error::NoTaskGiven(graph.known())),
            };

            render::explain(task, &loaded.root, cli.json);
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

            render::build(&report, cli.json);
            Ok(exit_on(report.succeeded()))
        }

        Command::Verify => {
            let engine = Engine::new(loaded.root.clone(), cache).cached(false);

            // A failed build has no artifact to compare.
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

            let reproducible = differing.is_empty();
            render::verify(&differing, graph.len(), cli.json);
            Ok(exit_on(reproducible))
        }

        Command::Watch { tasks, pick } => {
            let Some(selected) = choose(&graph, tasks, *pick, &loaded.root, "watch")? else {
                return Ok(ExitCode::Success);
            };

            // Resolved once, so a picked set survives the manifest being re-read.
            let names: Vec<String> = selected.tasks.iter().map(|t| t.id.clone()).collect();
            let scope = Scope::of(&selected, &cache)?;
            let json = cli.json;

            build::watch::run(&scope, &path, || {
                if let Err(error) = rebuild(&path, &cache, &names, json, &env, &overrides) {
                    eprintln!("error: {error}");
                }
            })?;

            Ok(ExitCode::Success)
        }

        Command::Schema { .. } | Command::Init { .. } => unreachable!("handled above"),
    }
}

fn exit_on(ok: bool) -> ExitCode {
    if ok {
        ExitCode::Success
    } else {
        ExitCode::Build
    }
}

/// One watch cycle: re-read, re-resolve, rebuild.
fn rebuild(
    path: &AbsPath,
    cache: &AbsPath,
    selectors: &[String],
    json: bool,
    env: &manifest::Env,
    overrides: &Overrides,
) -> Result<()> {
    let loaded = manifest::load(path, env)?;
    let resolution = plan::resolve(&loaded.manifest, &loaded.root, overrides);

    let Some(graph) = resolution.graph else {
        render::diags(&resolution.diags, json);
        return Ok(());
    };

    let report =
        Engine::new(loaded.root.clone(), cache.clone()).run(&select(&graph, selectors)?)?;
    render::build(&report, json);
    Ok(())
}

/// One `verify` pass. [`None`] when the build failed, which this reports first.
fn pass<'g>(
    engine: &Engine,
    graph: &'g Graph,
    json: bool,
) -> Result<Option<Vec<(&'g str, String)>>> {
    let report = engine.run(graph)?;

    if !report.succeeded() {
        render::build(&report, json);
        return Ok(None);
    }

    fingerprints(graph).map(Some)
}

/// A directory output folds in every file beneath it.
fn fingerprints(graph: &Graph) -> Result<Vec<(&str, String)>> {
    graph
        .tasks
        .iter()
        .map(|task| {
            let mut hasher = procmp::Hasher::new();

            for artifact in build::artifacts(&task.output)? {
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

/// [`None`] when the menu was dismissed.
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

    let Some(indices) = menu::tasks(graph, root, title, menu::Mode::Many)? else {
        return Ok(None);
    };

    Ok(Some(Graph {
        tasks: indices.iter().map(|at| graph.tasks[*at].clone()).collect(),
    }))
}

/// An empty selection is an error rather than a no-op.
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

/// `*` is the only wildcard, because a matrix identifier holds `[`, `]` and `=`. The
/// literals must appear in order: first anchored to the start, last to the end, the
/// rest anywhere between.
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

    // The tail is claimed before the middles are searched.
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

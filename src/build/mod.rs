//! Execution: [`Graph`] in, artifacts out.

mod scope;
pub mod watch;

pub use scope::Scope;

use std::collections::BTreeMap;
use std::time::Instant;

use darklua_core::{Configuration, Options, Resources};
use rayon::prelude::*;
use serde_json::{Map, Value};

use crate::digest::{self, Digest, Hasher};
use crate::error::{Error, Result};
use crate::manifest::Rule;
use crate::path::AbsPath;
use crate::plan::{Graph, Task};

/// Where build state lives when `--cache-dir` is not given.
pub const CACHE_DIR: &str = ".pcmp";

/// Extensions a header may be written to. A directory output can hold anything a `copy`
/// loader passed through.
const HEADABLE: &[&str] = &["luau", "lua"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum Outcome {
    Built { digest: String },
    Cached { digest: String },
    Failed { reason: String },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskReport {
    pub task: String,
    pub output: String,
    pub outcome: Outcome,
    pub millis: u128,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    pub tasks: Vec<TaskReport>,
}

impl Report {
    pub fn succeeded(&self) -> bool {
        !self
            .tasks
            .iter()
            .any(|t| matches!(t.outcome, Outcome::Failed { .. }))
    }

    /// Built, cached and failed.
    pub fn counts(&self) -> (usize, usize, usize) {
        self.tasks
            .iter()
            .fold((0, 0, 0), |(b, c, f), task| match task.outcome {
                Outcome::Built { .. } => (b + 1, c, f),
                Outcome::Cached { .. } => (b, c + 1, f),
                Outcome::Failed { .. } => (b, c, f + 1),
            })
    }
}

#[derive(Debug)]
pub struct Engine {
    root: AbsPath,
    cache: AbsPath,
    use_cache: bool,
}

impl Engine {
    pub fn new(root: AbsPath, cache: AbsPath) -> Self {
        Self {
            root,
            cache,
            use_cache: true,
        }
    }

    /// Disabled is what `verify` needs.
    #[must_use]
    pub fn cached(mut self, enabled: bool) -> Self {
        self.use_cache = enabled;
        self
    }

    /// Failures land in the report rather than returning early. Only whole-run problems
    /// return an error.
    pub fn run(&self, graph: &Graph) -> Result<Report> {
        // Before any work: two tasks writing one path would race.
        let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();
        for task in &graph.tasks {
            if let Some(first) = claimed.insert(task.output.as_str(), &task.id) {
                return Err(Error::OutputCollision(
                    first.to_owned(),
                    task.id.clone(),
                    task.output.to_string(),
                ));
            }
        }

        let sources = Scope::of(graph, &self.cache)?.fingerprint()?;

        let mut tasks: Vec<TaskReport> = graph
            .tasks
            .par_iter()
            .map(|task| {
                let started = Instant::now();
                let key = self.key(task, sources);

                TaskReport {
                    task: task.id.clone(),
                    output: task.output.relative_to(&self.root),
                    outcome: self.one(task, key).unwrap_or_else(|error| Outcome::Failed {
                        reason: error.to_string(),
                    }),
                    millis: started.elapsed().as_millis(),
                }
            })
            .collect();

        tasks.sort_by(|a, b| a.task.cmp(&b.task));
        Ok(Report { tasks })
    }

    /// darklua's version goes in: one manifest can produce different bytes against a
    /// different one.
    fn key(&self, task: &Task, sources: Digest) -> Digest {
        let mut h = Hasher::new();
        h.field("config", task.digest().bytes())
            .field("sources", sources.bytes())
            .field("darklua", crate::DARKLUA_VERSION);
        h.finish()
    }

    fn one(&self, task: &Task, key: Digest) -> Result<Outcome> {
        if !task.entry.exists() {
            return Err(Error::MissingEntry(
                task.id.clone(),
                task.entry.relative_to(&self.root),
            ));
        }

        // A directory output that exists but holds nothing is not a previous build.
        if self.use_cache
            && artifacts(&task.output).is_ok_and(|found| !found.is_empty())
            && self.fresh(task, key)
        {
            return Ok(Outcome::Cached {
                digest: key.short(),
            });
        }

        // A file output needs its parent. A directory output darklua creates itself.
        if task.entry.is_file()
            && let Some(parent) = task.output.parent()
        {
            std::fs::create_dir_all(parent.as_std())
                .map_err(|e| Error::Write(parent.to_string(), e.to_string()))?;
        }

        // `process` reports per-file failures inside the returned tree rather than as
        // an `Err`, so a build that produced nothing would look successful.
        let worked = darklua_core::process(
            &Resources::from_file_system(),
            Options::new(task.entry.as_std())
                .with_output(task.output.as_std())
                .with_configuration(configuration(task)?),
        )
        .map_err(|e| Error::Process(task.id.clone(), e.to_string()))?;

        let failures = worked.collect_errors();
        if !failures.is_empty() {
            let detail = failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            return Err(Error::Process(task.id.clone(), detail));
        }

        if !task.output.exists() {
            return Err(Error::NoOutput(
                task.id.clone(),
                task.output.relative_to(&self.root),
            ));
        }

        self.prepend_header(task)?;
        self.record(task, key)?;
        Ok(Outcome::Built {
            digest: key.short(),
        })
    }

    fn prepend_header(&self, task: &Task) -> Result<()> {
        if task.header.is_empty() {
            return Ok(());
        }

        let mut banner = task.header.join("\n");
        banner.push('\n');

        for artifact in artifacts(&task.output)? {
            if !artifact.extension().is_some_and(|e| HEADABLE.contains(&e)) {
                continue;
            }

            let path = artifact.as_std();
            let body = std::fs::read_to_string(path)
                .map_err(|e| Error::Read(artifact.to_string(), e.to_string()))?;

            std::fs::write(path, format!("{banner}{body}"))
                .map_err(|e| Error::Write(artifact.to_string(), e.to_string()))?;
        }

        Ok(())
    }

    /// Named by hash: a matrix identifier holds `[`, `]`, `=` and `,`.
    fn stamp(&self, task: &Task) -> Result<AbsPath> {
        self.cache
            .join(format!("{}.stamp", digest::of(&task.id).hex()))
    }

    /// A missing or unreadable stamp reads as stale.
    fn fresh(&self, task: &Task, key: Digest) -> bool {
        let Ok(path) = self.stamp(task) else {
            return false;
        };
        matches!(std::fs::read_to_string(path.as_std()), Ok(v) if v.trim() == key.hex())
    }

    fn record(&self, task: &Task, key: Digest) -> Result<()> {
        std::fs::create_dir_all(self.cache.as_std())
            .map_err(|e| Error::Write(self.cache.to_string(), e.to_string()))?;
        let path = self.stamp(task)?;
        std::fs::write(path.as_std(), key.hex())
            .map_err(|e| Error::Write(path.to_string(), e.to_string()))
    }
}

/// Every file an output covers: the file itself, or the tree beneath a directory.
pub fn artifacts(output: &AbsPath) -> Result<Vec<AbsPath>> {
    if output.is_file() {
        return Ok(vec![output.clone()]);
    }

    let mut found = Vec::new();
    let mut stack = vec![output.clone()];

    while let Some(dir) = stack.pop() {
        let read = match std::fs::read_dir(dir.as_std()) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(Error::Read(dir.to_string(), e.to_string())),
        };

        for entry in read {
            let entry = entry.map_err(|e| Error::Read(dir.to_string(), e.to_string()))?;
            let Ok(path) = dir.join(entry.file_name().to_string_lossy().as_ref()) else {
                continue;
            };

            match entry.file_type() {
                Ok(kind) if kind.is_dir() => stack.push(path),
                Ok(kind) if kind.is_file() => found.push(path),
                _ => {}
            }
        }
    }

    found.sort();
    Ok(found)
}

/// The manifest's own `darklua` block, with `rules` and `loaders` written over it.
pub fn config_json(task: &Task) -> Value {
    let mut config = task.darklua.clone();

    // Omitted rather than emitted empty: darklua reads a missing `rules` as "use the
    // defaults", which is not "run nothing".
    if let Some(rules) = &task.rules {
        config.insert(
            "rules".into(),
            Value::Array(rules.iter().map(Rule::json).collect()),
        );
    }

    if let Some(loaders) = &task.loaders {
        let map: Map<String, Value> = loaders
            .iter()
            .map(|l| (l.pattern.clone(), l.loader.clone().into()))
            .collect();
        config.insert("loaders".into(), Value::Object(map));
    }

    Value::Object(config)
}

/// On rejection the error carries the emitted JSON.
fn configuration(task: &Task) -> Result<Configuration> {
    let json = config_json(task);
    serde_json::from_value(json.clone()).map_err(|e| {
        Error::DarkluaConfig(
            task.id.clone(),
            e.to_string(),
            serde_json::to_string_pretty(&json).unwrap_or_default(),
        )
    })
}

//! Execution: [`Graph`] in, artifacts out.
//!
//! darklua is linked in rather than invoked, so there is no `PATH` lookup, no version
//! drift and no temp files. Tasks run in parallel and are content-addressed.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use darklua_core::{Configuration, Options, Resources};
use rayon::prelude::*;
use serde_json::{Map, Value};
use wax::{Glob, Program};

use crate::digest::{self, Digest, Hasher};
use crate::error::{Error, Result};
use crate::path::AbsPath;
use crate::plan::{Graph, Task};

/// Version-control metadata: never a build input, and it changes on every commit.
const VCS: &str = ".git";

/// Extensions a header may be written to.
///
/// A directory output can hold anything a `copy` loader passed through, so prepending
/// `--!native` to every file it produced would corrupt the images among them.
const HEADABLE: &[&str] = &["luau", "lua"];

/// What happened to one task.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum Outcome {
    /// Processed and written.
    Built {
        /// Short cache key the artifact was produced under.
        digest: String,
    },
    /// Inputs unchanged, so the existing artifact was kept.
    Cached {
        /// Short cache key that matched.
        digest: String,
    },
    /// Not produced. The reason is reported and other tasks still run.
    Failed {
        /// Why, rendered for the user.
        reason: String,
    },
}

/// One task's result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskReport {
    /// Task identifier, matching [`crate::plan::Task::id`].
    pub task: String,
    /// Output path, relative to the project root.
    pub output: String,
    /// What happened to it.
    pub outcome: Outcome,
    /// Wall-clock time spent on this task.
    pub millis: u128,
}

/// The result of one build, ordered by task identifier.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    /// One entry per task, ordered by identifier.
    pub tasks: Vec<TaskReport>,
}

impl Report {
    /// Whether every task produced or kept an artifact.
    pub fn succeeded(&self) -> bool {
        !self
            .tasks
            .iter()
            .any(|t| matches!(t.outcome, Outcome::Failed { .. }))
    }

    /// Built, cached and failed, in that order.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut built = 0;
        let mut cached = 0;
        let mut failed = 0;
        for task in &self.tasks {
            match task.outcome {
                Outcome::Built { .. } => built += 1,
                Outcome::Cached { .. } => cached += 1,
                Outcome::Failed { .. } => failed += 1,
            }
        }
        (built, cached, failed)
    }
}

/// The set of files a build reads, derived from the plan rather than guessed.
///
/// Every file under every task's `sources` counts, whatever its extension. A content
/// loader can make a `.json` or a `.png` a build input, and an extension allowlist
/// would serve a stale artifact after one changed. What is excluded is only what is
/// provably not an input: the cache, version control, and the artifacts themselves.
#[derive(Debug)]
pub struct Scope {
    roots: Vec<AbsPath>,
    excluded: BTreeSet<AbsPath>,
    ignore: Vec<Glob<'static>>,
}

impl Scope {
    /// Derives the scope of `graph`, with `cache` excluded.
    ///
    /// # Errors
    ///
    /// When an `ignore` entry is not a valid glob.
    pub fn of(graph: &Graph, cache: &AbsPath) -> Result<Self> {
        let mut roots = Vec::new();
        let mut excluded = BTreeSet::from([cache.clone()]);
        let mut patterns: BTreeSet<&str> = BTreeSet::new();

        for task in &graph.tasks {
            roots.extend(task.sources.iter().cloned());
            excluded.insert(task.output.clone());
            patterns.extend(task.ignore.iter().map(String::as_str));
        }

        roots.sort();
        roots.dedup();

        // A root nested inside another would hash its files twice under two different
        // relative names, so the shorter one wins.
        let outer = roots.clone();
        roots.retain(|root| !outer.iter().any(|other| root.is_within(other)));

        let ignore = patterns
            .into_iter()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(Glob::into_owned)
                    .map_err(|e| Error::BadGlob(pattern.to_owned(), e.to_string()))
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            roots,
            excluded,
            ignore,
        })
    }

    /// Whether an event path is a build input rather than output, state or ignored.
    ///
    /// A path that is not UTF-8 is not one of ours: every root came from an
    /// [`AbsPath`], so anything unrepresentable is outside the project by construction.
    pub fn contains(&self, path: &std::path::Path) -> bool {
        let Some(text) = path.to_str() else {
            return false;
        };
        let Ok(path) = AbsPath::new(text) else {
            return false;
        };

        self.holds(&path)
    }

    /// The same question against a path already parsed, which is what the walk has.
    fn holds(&self, path: &AbsPath) -> bool {
        let Some(root) = self.roots.iter().find(|root| path.is_under(root)) else {
            return false;
        };

        if self.excluded.iter().any(|excluded| path.is_under(excluded)) {
            return false;
        }

        let relative = path.relative_to(root);
        if relative.split('/').any(|segment| segment == VCS) {
            return false;
        }

        !self
            .ignore
            .iter()
            .any(|glob| glob.is_match(relative.as_str()))
    }

    /// Hashes every input, sorted, so filesystems that enumerate differently agree.
    ///
    /// A root that does not exist contributes nothing rather than failing. A missing
    /// entry point is reported per task, with its path.
    pub fn fingerprint(&self) -> Result<Digest> {
        let mut entries: BTreeMap<String, Digest> = BTreeMap::new();

        for root in &self.roots {
            self.walk(root, root, &mut entries)?;
        }

        let mut h = Hasher::new();
        h.seq(
            "sources",
            entries
                .iter()
                .map(|(path, d)| format!("{path}:{}", d.hex())),
        );
        Ok(h.finish())
    }

    fn walk(
        &self,
        root: &AbsPath,
        dir: &AbsPath,
        entries: &mut BTreeMap<String, Digest>,
    ) -> Result<()> {
        let read = match std::fs::read_dir(dir.as_std()) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(Error::Read(dir.to_string(), e.to_string())),
        };

        for entry in read {
            let entry = entry.map_err(|e| Error::Read(dir.to_string(), e.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();

            let Ok(path) = dir.join(&name) else { continue };
            if !self.holds(&path) {
                continue;
            }

            let kind = entry
                .file_type()
                .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;

            if kind.is_dir() {
                self.walk(root, &path, entries)?;
                continue;
            }

            // Symlinks are skipped: following them could leave the project.
            if !kind.is_file() {
                continue;
            }

            let bytes = std::fs::read(path.as_std())
                .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;
            entries.insert(path.relative_to(root), digest::of(&bytes));
        }

        Ok(())
    }

    /// The directories to hand a filesystem watcher.
    pub fn roots(&self) -> &[AbsPath] {
        &self.roots
    }
}

/// Where build state lives when `--cache-dir` is not given.
pub const CACHE_DIR: &str = ".pcmp";

/// Executes a build graph against the filesystem.
#[derive(Debug)]
pub struct Engine {
    root: AbsPath,
    cache: AbsPath,
    /// When false, every task is rebuilt regardless of recorded state.
    use_cache: bool,
}

impl Engine {
    /// Creates an engine rooted at `root`, keeping cache state in `cache`.
    pub fn new(root: AbsPath, cache: AbsPath) -> Self {
        Self {
            root,
            cache,
            use_cache: true,
        }
    }

    #[must_use]
    /// Enables or disables cache reuse. Disabled is what `pcmp verify` needs.
    pub fn cached(mut self, enabled: bool) -> Self {
        self.use_cache = enabled;
        self
    }

    /// Failures land in the report rather than returning early, so one broken profile
    /// does not hide the rest.
    ///
    /// # Errors
    ///
    /// Only for whole-run problems: two tasks claiming one output, an unreadable input,
    /// or an `ignore` entry that is not a valid glob.
    pub fn run(&self, graph: &Graph) -> Result<Report> {
        // Checked before any work starts. Two tasks writing one path would race, and
        // which one won would depend on thread scheduling.
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

                let outcome = match self.one(task, key) {
                    Ok(outcome) => outcome,
                    Err(error) => Outcome::Failed {
                        reason: error.to_string(),
                    },
                };

                TaskReport {
                    task: task.id.clone(),
                    output: task.output.relative_to(&self.root),
                    outcome,
                    millis: started.elapsed().as_millis(),
                }
            })
            .collect();

        // The pool finishes in any order, and this is printed, so sort it back.
        tasks.sort_by(|a, b| a.task.cmp(&b.task));
        Ok(Report { tasks })
    }

    /// Configuration, sources, and the darklua version. The last of those matters
    /// because the same manifest can legitimately produce different bytes against a
    /// different darklua.
    fn key(&self, task: &Task, sources: Digest) -> Digest {
        let mut h = Hasher::new();
        h.field("config", task.digest().bytes())
            .field("sources", sources.bytes())
            .field("darklua", crate::darklua_version());
        h.finish()
    }

    /// Builds one task, or reports it as already up to date.
    fn one(&self, task: &Task, key: Digest) -> Result<Outcome> {
        if !task.entry.exists() {
            return Err(Error::MissingEntry(
                task.id.clone(),
                task.entry.relative_to(&self.root),
            ));
        }

        if self.use_cache && task.output.exists() && self.fresh(task, key) {
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
        // an `Err`, so a build that produced nothing would otherwise look successful.
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

        // A backstop for the same class of problem: if darklua reports no failure but
        // writes nothing, that is still not a build. A file filter that matches nothing
        // is by far the likeliest cause, and the silence is otherwise baffling.
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

    /// Writes the task's header lines above every artifact it produced.
    ///
    /// Done here rather than through darklua's `append_text_comment` rule because the
    /// `dense` and `readable` generators discard comments, which would silently drop a
    /// `--!native` directive from exactly the builds that want it.
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

    /// Named by hash: matrix identifiers contain `[`, `]`, `=` and `,`, which are not
    /// portable in filenames.
    fn stamp(&self, task: &Task) -> Result<AbsPath> {
        self.cache
            .join(format!("{}.stamp", digest::of(&task.id).hex()))
    }

    /// Reports whether a recorded stamp matches this task's current key.
    fn fresh(&self, task: &Task, key: Digest) -> bool {
        let Ok(path) = self.stamp(task) else {
            return false;
        };
        // A missing or unreadable stamp reads as stale. A needless rebuild costs
        // seconds, a wrong cache hit costs correctness.
        matches!(std::fs::read_to_string(path.as_std()), Ok(v) if v.trim() == key.hex())
    }

    /// Records a task's key so the next run can skip it.
    fn record(&self, task: &Task, key: Digest) -> Result<()> {
        std::fs::create_dir_all(self.cache.as_std())
            .map_err(|e| Error::Write(self.cache.to_string(), e.to_string()))?;
        let path = self.stamp(task)?;
        std::fs::write(path.as_std(), key.hex())
            .map_err(|e| Error::Write(path.to_string(), e.to_string()))
    }
}

/// Every file an output covers: the file itself, or the tree beneath a directory.
///
/// Sorted, so hashing a directory output is order-independent.
///
/// # Errors
///
/// When a directory under `output` cannot be read.
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

/// Renders a task as a darklua configuration.
///
/// Mostly the manifest's own `darklua` block: only `rules`, which carries ProCMP's
/// injections, and `loaders`, which a manifest format cannot order, are written here.
/// `pcmp explain` prints the result, so it doubles as a config a user can paste
/// elsewhere.
pub fn config_json(task: &Task) -> Value {
    let mut config = task.darklua.clone();

    // Omitted rather than emitted empty when the task declares none: darklua reads a
    // missing `rules` as "use the defaults", which is not the same as "run nothing".
    if let Some(rules) = &task.rules {
        config.insert(
            "rules".into(),
            Value::Array(rules.iter().map(|rule| rule.json()).collect()),
        );
    }

    if let Some(loaders) = &task.loaders {
        let mut map = Map::new();
        for loader in loaders {
            map.insert(loader.pattern.clone(), loader.loader.clone().into());
        }
        config.insert("loaders".into(), Value::Object(map));
    }

    Value::Object(config)
}

/// On rejection the error carries the emitted JSON, so the reader sees what was
/// produced rather than only that darklua disliked it.
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

//! Resolution: [`Manifest`] in, [`Graph`] out.
//!
//! Pure: no filesystem, no network, no clock. `pcmp plan` is therefore exactly what a
//! build would do rather than an approximation. Findings accumulate rather than
//! short-circuit, so a manifest with four mistakes reports all four in one run.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::diag::{self, Diag};
use crate::digest::{Digest, Hasher};
use crate::error::{Error, Result};
use crate::manifest::{Define, Loader, Manifest, Matrix, Profile, Rule};
use crate::path::AbsPath;
use crate::rules;

/// `extends` names a profile that does not exist.
pub const UNKNOWN_BASE: &str = "unknown-base";
/// An `extends` chain forms a cycle.
pub const CYCLIC_EXTENDS: &str = "cyclic-extends";
/// A concrete profile has no `entry` after inheritance.
pub const MISSING_ENTRY: &str = "missing-entry";
/// A concrete profile has no `output` after inheritance.
pub const MISSING_OUTPUT: &str = "missing-output";
/// An output, header or var template has an unknown, empty, or unclosed token.
pub const BAD_TEMPLATE: &str = "bad-template";
/// A path in `entry`, `output` or `sources` cannot be resolved.
pub const BAD_PATH: &str = "bad-path";
/// A matrix names a base profile that does not exist.
pub const UNKNOWN_MATRIX_BASE: &str = "unknown-matrix-base";
/// A matrix axis has no values, so it expands to nothing.
pub const EMPTY_AXIS: &str = "empty-axis";
/// A define has a name or a value that cannot become a literal in source.
pub const BAD_DEFINE: &str = "bad-define";
/// `darklua.rules` is not a list darklua could read.
pub const BAD_RULES: &str = "bad-rules";
/// Loaders declared in both places, where only one can win.
pub const DARKLUA_LOADERS: &str = "darklua-loaders";
/// A var name cannot become a token or a constant, or two collide.
pub const BAD_VAR: &str = "bad-var";
/// A matrix axis lists the same value twice.
pub const DUPLICATE_AXIS_VALUE: &str = "duplicate-axis-value";
/// The manifest declares neither profiles nor matrices.
pub const NO_TASKS: &str = "no-tasks";

/// Luau words that cannot be used as a name.
const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// One unit of work, fully resolved: no inherited fields, no relative paths, no
/// unexpanded templates.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Task {
    /// A profile name, or `dist[flavour=min,target=roblox]` for a matrix task.
    pub id: String,
    /// The `profiles` or `matrix` key this task came from. `pcmp build` accepts it as
    /// a selector, so every task a matrix expands into is buildable by one name.
    pub profile: String,
    /// Absolute path darklua reads: a file, or a directory processed as a tree.
    pub entry: AbsPath,
    /// Absolute path darklua writes, with every template token expanded.
    pub output: AbsPath,
    /// Constants injected before folding, including the built-in `PCMP_*` set.
    pub defines: IndexMap<String, Define>,
    /// Token values available to `output` and `header`.
    pub vars: IndexMap<String, String>,
    /// The rules darklua will run, or [`None`] to let it use its own defaults.
    pub rules: Option<Vec<Rule>>,
    /// Content loaders, in first-match-wins order.
    pub loaders: Option<Vec<Loader>>,
    /// The rest of darklua's configuration, passed through untouched.
    pub darklua: Map<String, Value>,
    /// Directories whose contents are build inputs, the manifest's own included.
    pub sources: Vec<AbsPath>,
    /// Globs excluded from that input set.
    pub ignore: Vec<String>,
    /// Lines prepended to each artifact after darklua runs, tokens expanded.
    pub header: Vec<String>,
    /// Matrix coordinates that produced this task. Empty for a plain profile.
    pub axes: IndexMap<String, String>,
}

impl Task {
    /// Fingerprints everything affecting output except the source bytes, which
    /// [`crate::engine`] folds in separately. A field omitted here is a field that can
    /// change without invalidating the cache.
    pub fn digest(&self) -> Digest {
        let mut h = Hasher::new();

        h.field("id", &self.id)
            .field("entry", self.entry.as_str())
            .field("output", self.output.as_str())
            .seq(
                "defines",
                self.defines
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.tagged())),
            )
            .field("rules", json(&self.rules))
            .field("loaders", json(&self.loaders))
            .field("darklua", json(&self.darklua))
            .seq("header", &self.header);

        h.finish()
    }
}

/// Renders a value for hashing. Every input here is plain data that already serialises,
/// so a failure would be a bug rather than a user's mistake.
fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("plan data is always encodable")
}

/// The resolved plan.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Graph {
    /// Every task, ordered by identifier.
    pub tasks: Vec<Task>,
}

impl Graph {
    fn sorted(mut tasks: Vec<Task>) -> Self {
        tasks.sort_by(|a, b| a.id.cmp(&b.id));
        Self { tasks }
    }

    /// How many tasks the plan holds.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether the plan holds no tasks.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// The task with this identifier, or [`None`].
    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// Every task identifier, rendered for a "did you mean" message.
    pub fn known(&self) -> String {
        join(self.tasks.iter().map(|task| task.id.as_str()))
    }

    /// Fingerprints the whole plan, so two runs can be compared at a glance.
    pub fn digest(&self) -> Digest {
        let mut h = Hasher::new();
        h.seq("tasks", self.tasks.iter().map(|t| t.digest().hex()));
        h.finish()
    }
}

/// Warnings accompany a usable graph. Any denial suppresses it.
#[derive(Debug)]
pub struct Resolution {
    /// The plan, or [`None`] when a finding denied it.
    pub graph: Option<Graph>,
    /// Everything found while resolving, sorted.
    pub diags: Vec<Diag>,
}

/// Values supplied with `--define` and `--var`, applied after inheritance so they beat
/// anything the manifest says without the manifest having to anticipate them.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// Constants from `--define`.
    pub defines: IndexMap<String, Define>,
    /// Tokens from `--var`.
    pub vars: IndexMap<String, String>,
}

impl Overrides {
    /// Parses `KEY=VALUE` arguments. A repeated key takes its last value.
    ///
    /// # Errors
    ///
    /// When an argument has no `=`.
    pub fn parse(defines: &[String], vars: &[String]) -> Result<Self> {
        Ok(Self {
            defines: pairs(defines)?
                .into_iter()
                .map(|(key, value)| (key, Define::parse(&value)))
                .collect(),
            vars: pairs(vars)?.into_iter().collect(),
        })
    }
}

fn pairs(arguments: &[String]) -> Result<IndexMap<String, String>> {
    arguments
        .iter()
        .map(|argument| {
            argument
                .split_once('=')
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .ok_or_else(|| Error::BadPair(argument.clone()))
        })
        .collect()
}

/// Resolves a manifest into a build graph rooted at `root`.
pub fn resolve(manifest: &Manifest, root: &AbsPath, overrides: &Overrides) -> Resolution {
    let mut diags = Vec::new();
    let mut tasks = Vec::new();

    if manifest.profiles.is_empty() && manifest.matrix.is_empty() {
        diags.push(
            Diag::deny(NO_TASKS, "this manifest defines no build tasks")
                .help("add a `profiles` entry, or a `matrix` that expands into one"),
        );
    }

    for (name, declared) in &manifest.profiles {
        // A template is not a build. Resolving it would demand an `entry` and an
        // `output` it has no reason to own.
        if declared.is_abstract {
            continue;
        }

        if let Some(profile) = flatten(manifest, name, &mut diags)
            && let Some(task) = task(
                name,
                &profile,
                &IndexMap::new(),
                root,
                overrides,
                &mut diags,
            )
        {
            tasks.push(task);
        }
    }

    for (name, matrix) in &manifest.matrix {
        expand(
            manifest, name, matrix, root, overrides, &mut tasks, &mut diags,
        );
    }

    diag::sort(&mut diags);

    let denied = diag::worst(&diags) == Some(diag::Severity::Deny);
    Resolution {
        graph: (!denied).then(|| Graph::sorted(tasks)),
        diags,
    }
}

/// Collapses an `extends` chain into a single profile.
///
/// Nearer profiles win field by field. `vars` and `define` are the exceptions and
/// accumulate, so a profile adding one entry keeps the ones it inherited.
fn flatten(manifest: &Manifest, name: &str, diags: &mut Vec<Diag>) -> Option<Profile> {
    let mut chain: Vec<&Profile> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut cursor = name;

    loop {
        // Also bounds the walk: a chain longer than the profile count must repeat a
        // name and land here, so no separate depth limit is needed.
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

    // The manifest's own vars are the root of every chain.
    let mut merged = Profile {
        vars: manifest.vars.clone(),
        ..Profile::default()
    };

    // `chain` runs child to ancestor, so fold from the ancestor end and nearer wins.
    for profile in chain.iter().rev() {
        merge(&mut merged, profile);
    }
    merged.extends = None;
    // Abstractness is not inherited: extending a template produces a real build.
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

    // Merged key by key rather than replaced, so a profile can set `generator` without
    // restating the `bundle` it inherited.
    if let Some(overlay) = &overlay.darklua {
        let merged = base.darklua.get_or_insert_with(Map::new);
        for (key, value) in overlay {
            merged.insert(key.clone(), value.clone());
        }
    }

    for (key, value) in &overlay.vars {
        base.vars.insert(key.clone(), value.clone());
    }
    base.vars.sort_keys();

    for (key, value) in &overlay.define {
        base.define.insert(key.clone(), value.clone());
    }
    base.define.sort_keys();
}

fn known(manifest: &Manifest) -> String {
    join(manifest.profiles.keys().map(String::as_str))
}

/// Renders a name list for a "did you mean" message.
fn join<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let joined = names.collect::<Vec<_>>().join(", ");
    if joined.is_empty() {
        "<none>".into()
    } else {
        joined
    }
}

fn id_of(profile: &str, axes: &IndexMap<String, String>) -> String {
    if axes.is_empty() {
        return profile.to_owned();
    }
    // Axes arrive key-sorted, so coordinates render identically however listed.
    let coords = axes
        .iter()
        .map(|(axis, value)| format!("{axis}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{profile}[{coords}]")
}

/// Turns one flattened profile into a task, or records why it cannot become one.
fn task(
    profile_name: &str,
    profile: &Profile,
    axes: &IndexMap<String, String>,
    root: &AbsPath,
    overrides: &Overrides,
    diags: &mut Vec<Diag>,
) -> Option<Task> {
    let id = id_of(profile_name, axes);

    let Some(entry) = profile.entry.as_deref() else {
        diags.push(
            Diag::deny(MISSING_ENTRY, format!("task `{id}` has no `entry`"))
                .help("set `entry` on the profile, or on one it extends"),
        );
        return None;
    };

    let Some(template) = profile.output.as_deref() else {
        diags.push(
            Diag::deny(MISSING_OUTPUT, format!("task `{id}` has no `output`")).help(
                "set `output`, e.g. \"dist/{profile}/app.luau\"\n  \
                 ProCMP does not invent one, because an artifact written somewhere you \
                 did not ask for is an artifact you will not find",
            ),
        );
        return None;
    };

    // `--var` last, so a command line beats both the manifest and the matrix.
    let mut vars = profile.vars.clone();
    for (key, value) in axes {
        vars.insert(key.clone(), value.clone());
    }
    for (key, value) in &overrides.vars {
        vars.insert(key.clone(), value.clone());
    }
    vars.insert("profile".into(), profile_name.to_owned());
    vars.sort_keys();

    let mut constants: IndexMap<String, &str> = IndexMap::new();
    for name in vars.keys() {
        if !is_identifier(name) {
            diags.push(
                Diag::deny(
                    BAD_VAR,
                    format!("var `{name}` in task `{id}` is not a name"),
                )
                .help(
                    "a var becomes both a `{token}` and a `PCMP_<NAME>` constant, so it \
                     has to be writable as a Luau identifier",
                ),
            );
            return None;
        }

        // `PCMP_<NAME>` is uppercased, so `channel` and `Channel` would arrive as one
        // constant and the second would quietly replace the first.
        let constant = name.to_uppercase();
        if let Some(first) = constants.insert(constant.clone(), name.as_str()) {
            diags.push(
                Diag::deny(
                    BAD_VAR,
                    format!("vars `{first}` and `{name}` in task `{id}` share a constant"),
                )
                .help(format!("both become `PCMP_{constant}`, so rename one")),
            );
            return None;
        }
    }

    let relative = match expand_tokens(template, &vars) {
        Ok(expanded) => expanded,
        Err(error) => {
            diags.push(
                Diag::deny(
                    BAD_TEMPLATE,
                    format!("task `{id}` has an unusable `output` template"),
                )
                .help(error.to_string()),
            );
            return None;
        }
    };

    let entry = resolve_path(root, entry, &id, "entry", diags)?;
    let output = resolve_path(root, &relative, &id, "output", diags)?;

    let mut sources = vec![root.clone()];
    for extra in profile.sources.iter().flatten() {
        sources.push(resolve_path(root, extra, &id, "sources", diags)?);
    }
    sources.sort();
    sources.dedup();

    let mut defines = builtins(&vars, axes);
    for (key, value) in &profile.define {
        defines.insert(key.clone(), value.clone());
    }
    for (key, value) in &overrides.defines {
        defines.insert(key.clone(), value.clone());
    }
    defines.sort_keys();

    for (identifier, value) in &defines {
        // `inject_global_value` substitutes by name. A key that cannot be written as a
        // global matches nothing, so it would be a define that silently does nothing.
        if !is_identifier(identifier) {
            diags.push(
                Diag::deny(
                    BAD_DEFINE,
                    format!("define `{identifier}` in task `{id}` is not a Luau identifier"),
                )
                .help(
                    "a define key must be writable as a global: letters, digits and \
                     underscores, not starting with a digit, and not a keyword",
                ),
            );
            return None;
        }

        if matches!(value, Define::Number(number) if !number.is_finite()) {
            diags.push(
                Diag::deny(
                    BAD_DEFINE,
                    format!("define `{identifier}` in task `{id}` is not a finite number"),
                )
                .help(
                    "infinity and NaN have no literal form, so use a string if that was the intent",
                ),
            );
            return None;
        }
    }

    let mut header = Vec::new();
    for line in profile.header.iter().flatten() {
        match expand_tokens(line, &vars) {
            Ok(expanded) => header.push(expanded),
            Err(error) => {
                diags.push(
                    Diag::deny(
                        BAD_TEMPLATE,
                        format!("task `{id}` has an unusable `header` line"),
                    )
                    .help(error.to_string()),
                );
                return None;
            }
        }
    }

    let mut darklua = profile.darklua.clone().unwrap_or_default();

    if darklua.contains_key("loaders") && profile.loaders.is_some() {
        diags.push(
            Diag::deny(
                DARKLUA_LOADERS,
                format!("task `{id}` declares loaders in both `loaders` and `darklua.loaders`"),
            )
            .help("keep the `loaders` list, because only it can express which pattern wins"),
        );
        return None;
    }

    // Lifted out because ProCMP has to place injections ahead of whatever was declared.
    // Everything else in the block stays untouched.
    let declared = match darklua.remove("rules") {
        None => None,
        Some(value) => match serde_json::from_value::<Vec<Rule>>(value) {
            Ok(rules) => Some(rules),
            Err(error) => {
                diags.push(
                    Diag::deny(
                        BAD_RULES,
                        format!("task `{id}` has an unreadable `darklua.rules`"),
                    )
                    .help(error.to_string()),
                );
                return None;
            }
        },
    };

    Some(Task {
        id,
        profile: profile_name.to_owned(),
        entry,
        output,
        rules: rules::assemble(&defines, declared.as_deref()),
        defines,
        vars,
        loaders: profile.loaders.clone(),
        darklua,
        sources,
        ignore: profile.ignore.clone().unwrap_or_default(),
        header,
        axes: axes.clone(),
    })
}

fn resolve_path(
    root: &AbsPath,
    path: &str,
    id: &str,
    field: &str,
    diags: &mut Vec<Diag>,
) -> Option<AbsPath> {
    match root.join(path) {
        Ok(resolved) => Some(resolved),
        Err(error) => {
            diags.push(
                Diag::deny(
                    BAD_PATH,
                    format!("task `{id}`: bad `{field}` path `{path}`"),
                )
                .help(error.to_string()),
            );
            None
        }
    }
}

/// Constants contributed to every task: one per var, one per axis. Ordinary defines,
/// so they fold like any other, and a `define` of the same name overrides them.
fn builtins(
    vars: &IndexMap<String, String>,
    axes: &IndexMap<String, String>,
) -> IndexMap<String, Define> {
    vars.iter()
        .chain(axes)
        .map(|(name, value)| {
            (
                format!("PCMP_{}", name.to_uppercase()),
                Define::Text(value.clone()),
            )
        })
        .collect()
}

/// Expands a matrix into one task per axis combination.
fn expand(
    manifest: &Manifest,
    name: &str,
    matrix: &Matrix,
    root: &AbsPath,
    overrides: &Overrides,
    tasks: &mut Vec<Task>,
    diags: &mut Vec<Diag>,
) {
    if !manifest.profiles.contains_key(&matrix.base) {
        diags.push(
            Diag::deny(
                UNKNOWN_MATRIX_BASE,
                format!("matrix `{name}` extends unknown profile `{}`", matrix.base),
            )
            .help(format!("known: {}", known(manifest))),
        );
        return;
    }

    for (axis, values) in &matrix.axes {
        if values.is_empty() {
            diags.push(
                Diag::deny(
                    EMPTY_AXIS,
                    format!("matrix `{name}` axis `{axis}` has no values"),
                )
                .help("an empty axis expands to zero tasks, which is never intended"),
            );
            return;
        }

        // A repeat would expand to two tasks with identical coordinates, which then
        // collide on their output path and fail the whole run rather than this axis.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        if let Some(repeated) = values.iter().find(|value| !seen.insert(value.as_str())) {
            diags.push(
                Diag::deny(
                    DUPLICATE_AXIS_VALUE,
                    format!("matrix `{name}` axis `{axis}` lists `{repeated}` twice"),
                )
                .help("each value expands to one task, so a repeat has no second meaning"),
            );
            return;
        }
    }

    let Some(base) = flatten(manifest, &matrix.base, diags) else {
        return;
    };

    for combination in combinations(&matrix.axes) {
        let mut profile = base.clone();

        if let Some(output) = matrix.output.as_deref() {
            profile.output = Some(output.to_owned());
        }
        for (key, value) in &matrix.define {
            profile.define.insert(key.clone(), value.clone());
        }
        for (key, value) in &matrix.vars {
            profile.vars.insert(key.clone(), value.clone());
        }
        profile.define.sort_keys();
        profile.vars.sort_keys();

        if let Some(task) = task(name, &profile, &combination, root, overrides, diags) {
            tasks.push(task);
        }
    }
}

/// Cartesian product of the axes. Sorted axes and declared value order make expansion
/// identical on every machine.
fn combinations(axes: &IndexMap<String, Vec<String>>) -> Vec<IndexMap<String, String>> {
    let mut result: Vec<IndexMap<String, String>> = vec![IndexMap::new()];

    for (axis, values) in axes {
        let mut next = Vec::with_capacity(result.len() * values.len());
        for partial in &result {
            for value in values {
                let mut extended = partial.clone();
                extended.insert(axis.clone(), value.clone());
                next.push(extended);
            }
        }
        result = next;
    }

    result
}

/// Whether `name` can appear as a global in Luau source.
fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();

    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
        && !KEYWORDS.contains(&name)
}

/// Expands `{token}` references, treating `{{` and `}}` as literal braces.
///
/// An unknown, empty or unclosed token is an error rather than a placeholder. A
/// template that quietly resolves to nothing names an artifact after nothing.
fn expand_tokens(template: &str, tokens: &IndexMap<String, String>) -> Result<String> {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => {
                out.push('{');
                i += 2;
            }
            b'}' if bytes.get(i + 1) == Some(&b'}') => {
                out.push('}');
                i += 2;
            }
            b'{' => {
                let start = i + 1;
                let end = template[start..]
                    .find('}')
                    .map(|offset| start + offset)
                    .ok_or_else(|| Error::UnterminatedToken(template.into()))?;

                let token = &template[start..end];
                let value = tokens.get(token).ok_or_else(|| {
                    let mut names: Vec<_> = tokens.keys().map(|k| format!("{{{k}}}")).collect();
                    names.sort();
                    Error::UnknownToken(token.into(), names.join(", "))
                })?;

                if value.is_empty() {
                    return Err(Error::EmptyToken(token.into()));
                }

                out.push_str(value);
                i = end + 1;
            }
            // An unpaired `}` is not ambiguous the way `{` is, so it passes through.
            b'}' => {
                out.push('}');
                i += 1;
            }
            _ => {
                let start = i;
                while i < bytes.len() && bytes[i] != b'{' && bytes[i] != b'}' {
                    i += 1;
                }
                out.push_str(&template[start..i]);
            }
        }
    }

    Ok(out)
}

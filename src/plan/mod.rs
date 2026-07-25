//! Resolution: [`Manifest`] in, [`Graph`] out.
//!
//! Pure: no filesystem, no network, no clock. Findings accumulate rather than
//! short-circuit.

mod inherit;
mod matrix;
pub mod rules;
mod token;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::diag::{self, Diag};
use crate::digest::{Digest, Hasher};
use crate::error::{Error, Result};
use crate::manifest::{Define, Loader, Manifest, Profile, Rule};
use crate::path::AbsPath;

use inherit::flatten;
use token::{expand, is_identifier};

pub const UNKNOWN_BASE: &str = "unknown-base";
pub const CYCLIC_EXTENDS: &str = "cyclic-extends";
pub const MISSING_ENTRY: &str = "missing-entry";
pub const MISSING_OUTPUT: &str = "missing-output";
pub const BAD_TEMPLATE: &str = "bad-template";
pub const BAD_PATH: &str = "bad-path";
pub const UNKNOWN_MATRIX_BASE: &str = "unknown-matrix-base";
pub const EMPTY_AXIS: &str = "empty-axis";
pub const DUPLICATE_AXIS_VALUE: &str = "duplicate-axis-value";
pub const BAD_DEFINE: &str = "bad-define";
pub const BAD_VAR: &str = "bad-var";
pub const BAD_RULES: &str = "bad-rules";
pub const DARKLUA_LOADERS: &str = "darklua-loaders";
pub const NO_TASKS: &str = "no-tasks";

/// One unit of work: no inherited fields, no relative paths, no unexpanded templates.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Task {
    /// A profile name, or `dist[flavour=min,target=roblox]` for a matrix task.
    pub id: String,
    /// The `profiles` or `matrix` key this came from, which `build` accepts as a
    /// selector.
    pub profile: String,
    /// A file, or a directory processed as a tree.
    pub entry: AbsPath,
    pub output: AbsPath,
    pub defines: IndexMap<String, Define>,
    pub vars: IndexMap<String, String>,
    /// [`None`] lets darklua use its own defaults.
    pub rules: Option<Vec<Rule>>,
    /// First match wins.
    pub loaders: Option<Vec<Loader>>,
    /// The rest of darklua's configuration, untouched.
    pub darklua: Map<String, Value>,
    /// Directories whose contents are build inputs.
    pub sources: Vec<AbsPath>,
    pub ignore: Vec<String>,
    pub header: Vec<String>,
    pub axes: IndexMap<String, String>,
}

impl Task {
    /// Everything affecting output except the source bytes, which [`crate::build`]
    /// folds in separately.
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

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("plan data is always encodable")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Graph {
    pub tasks: Vec<Task>,
}

impl Graph {
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// Every identifier, for a "did you mean" message.
    pub fn known(&self) -> String {
        join(self.tasks.iter().map(|task| task.id.as_str()))
    }

    pub fn digest(&self) -> Digest {
        let mut h = Hasher::new();
        h.seq("tasks", self.tasks.iter().map(|t| t.digest().hex()));
        h.finish()
    }
}

/// Warnings accompany a usable graph. Any denial suppresses it.
#[derive(Debug)]
pub struct Resolution {
    pub graph: Option<Graph>,
    pub diags: Vec<Diag>,
}

/// `--define` and `--var`, applied after inheritance so they beat the manifest without
/// it having to anticipate them.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub defines: IndexMap<String, Define>,
    pub vars: IndexMap<String, String>,
}

impl Overrides {
    pub fn parse(defines: &[String], vars: &[String]) -> Result<Self> {
        Ok(Self {
            defines: pairs(defines)?
                .into_iter()
                .map(|(key, value)| (key, Define::parse(&value)))
                .collect(),
            vars: pairs(vars)?,
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
        // A template is not a build.
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

    for (name, spec) in &manifest.matrix {
        matrix::expand(
            manifest, name, spec, root, overrides, &mut tasks, &mut diags,
        );
    }

    diag::sort(&mut diags);
    tasks.sort_by(|a, b| a.id.cmp(&b.id));

    let denied = diag::worst(&diags) == Some(diag::Severity::Deny);
    Resolution {
        graph: (!denied).then_some(Graph { tasks }),
        diags,
    }
}

fn known(manifest: &Manifest) -> String {
    join(manifest.profiles.keys().map(String::as_str))
}

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
    let coords = axes
        .iter()
        .map(|(axis, value)| format!("{axis}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{profile}[{coords}]")
}

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
            Diag::deny(MISSING_OUTPUT, format!("task `{id}` has no `output`"))
                .help("set `output`, e.g. \"dist/{profile}/app.luau\""),
        );
        return None;
    };

    let vars = vars(profile, axes, overrides, profile_name, &id, diags)?;
    let defines = defines(profile, &vars, axes, overrides, &id, diags)?;

    let relative = template_of(template, &vars, &id, "output", diags)?;
    let entry = path_of(root, entry, &id, "entry", diags)?;
    let output = path_of(root, &relative, &id, "output", diags)?;

    let mut sources = vec![root.clone()];
    for extra in profile.sources.iter().flatten() {
        sources.push(path_of(root, extra, &id, "sources", diags)?);
    }
    sources.sort();
    sources.dedup();

    let mut header = Vec::new();
    for line in profile.header.iter().flatten() {
        header.push(template_of(line, &vars, &id, "header", diags)?);
    }

    let mut darklua = profile.darklua.clone().unwrap_or_default();

    if darklua.contains_key("loaders") && profile.loaders.is_some() {
        diags.push(
            Diag::deny(
                DARKLUA_LOADERS,
                format!("task `{id}` declares loaders in both `loaders` and `darklua.loaders`"),
            )
            .help("keep the `loaders` list, which decides the order patterns match in"),
        );
        return None;
    }

    // Lifted out because injections go ahead of whatever was declared.
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

/// `--var` last, so a command line beats both the manifest and the matrix.
fn vars(
    profile: &Profile,
    axes: &IndexMap<String, String>,
    overrides: &Overrides,
    profile_name: &str,
    id: &str,
    diags: &mut Vec<Diag>,
) -> Option<IndexMap<String, String>> {
    let mut vars = profile.vars.clone();
    vars.extend(axes.iter().map(|(k, v)| (k.clone(), v.clone())));
    vars.extend(overrides.vars.iter().map(|(k, v)| (k.clone(), v.clone())));
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
                .help("a var becomes a `{token}` and a `PCMP_<NAME>` Luau identifier"),
            );
            return None;
        }

        // `PCMP_<NAME>` is uppercased, so `channel` and `Channel` collide.
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

    Some(vars)
}

/// One constant per var and per axis. Ordinary defines, so a `define` of the same name
/// overrides them.
fn defines(
    profile: &Profile,
    vars: &IndexMap<String, String>,
    axes: &IndexMap<String, String>,
    overrides: &Overrides,
    id: &str,
    diags: &mut Vec<Diag>,
) -> Option<IndexMap<String, Define>> {
    let mut defines: IndexMap<String, Define> = vars
        .iter()
        .chain(axes)
        .map(|(name, value)| {
            (
                format!("PCMP_{}", name.to_uppercase()),
                Define::Text(value.clone()),
            )
        })
        .collect();

    defines.extend(profile.define.iter().map(|(k, v)| (k.clone(), v.clone())));
    defines.extend(
        overrides
            .defines
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    defines.sort_keys();

    for (identifier, value) in &defines {
        // `inject_global_value` substitutes by name.
        if !is_identifier(identifier) {
            diags.push(
                Diag::deny(
                    BAD_DEFINE,
                    format!("define `{identifier}` in task `{id}` is not a Luau identifier"),
                )
                .help(
                    "letters, digits and underscores, not starting with a digit, and \
                     not a keyword",
                ),
            );
            return None;
        }

        if matches!(value, Define::Number(n) if !n.is_finite()) {
            diags.push(
                Diag::deny(
                    BAD_DEFINE,
                    format!("define `{identifier}` in task `{id}` is not a finite number"),
                )
                .help("infinity and NaN have no Luau literal, so use a string"),
            );
            return None;
        }
    }

    Some(defines)
}

fn template_of(
    template: &str,
    vars: &IndexMap<String, String>,
    id: &str,
    field: &str,
    diags: &mut Vec<Diag>,
) -> Option<String> {
    expand(template, vars)
        .inspect_err(|error| {
            diags.push(
                Diag::deny(
                    BAD_TEMPLATE,
                    format!("task `{id}` has an unusable `{field}` template"),
                )
                .help(error.to_string()),
            );
        })
        .ok()
}

fn path_of(
    root: &AbsPath,
    path: &str,
    id: &str,
    field: &str,
    diags: &mut Vec<Diag>,
) -> Option<AbsPath> {
    root.join(path)
        .inspect_err(|error| {
            diags.push(
                Diag::deny(
                    BAD_PATH,
                    format!("task `{id}`: bad `{field}` path `{path}`"),
                )
                .help(error.to_string()),
            );
        })
        .ok()
}

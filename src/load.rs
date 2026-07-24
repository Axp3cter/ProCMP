//! Manifest discovery and parsing.
//!
//! Format comes from the extension, never from content, so the same bytes cannot mean
//! different things in different directories.
//!
//! A Luau manifest is a program, and stays reproducible because the interpreter has
//! nothing nondeterministic left in it. Luau's sandbox removes `os`, `io`, `load`,
//! `dofile`, `debug` and `coroutine`, and [`REVOKED`] removes the rest. Its only
//! channel outward is `pcmp.env`

use mlua::{Lua, LuaOptions, LuaSerdeExt, StdLib, Value, VmState};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::path::AbsPath;

/// Manifest file names, in discovery order.
pub const CANDIDATES: &[&str] = &[
    "pcmp.luau",
    "pcmp.json",
    "pcmp.jsonc",
    "pcmp.json5",
    "pcmp.toml",
];

/// Globals that survive Luau's sandbox but would let a manifest observe the outside,
/// load code at runtime, or return a different value on a second evaluation.
pub const REVOKED: &[&str] = &[
    "collectgarbage",
    "getfenv",
    "loadstring",
    "newproxy",
    "require",
    "setfenv",
];

/// Entropy inside libraries that are otherwise safe to keep.
const REVOKED_MEMBERS: &[(&str, &str)] = &[("math", "random"), ("math", "randomseed")];

/// What `pcmp.env` reads: values given with `--env`, then the process environment.
///
/// Passed explicitly rather than written into the process environment, so a value given
/// on the command line cannot leak into anything ProCMP spawns.
#[derive(Debug, Clone, Default)]
pub struct Env(Vec<(String, String)>);

impl Env {
    /// Parses `KEY=VALUE` arguments, last occurrence of a key winning.
    ///
    /// # Errors
    ///
    /// When an argument has no `=`.
    pub fn parse(arguments: &[String]) -> Result<Self> {
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(arguments.len());

        for argument in arguments {
            let (key, value) = argument
                .split_once('=')
                .ok_or_else(|| Error::BadPair(argument.clone()))?;

            pairs.retain(|(existing, _)| existing != key);
            pairs.push((key.to_owned(), value.to_owned()));
        }

        Ok(Self(pairs))
    }

    fn get(&self, name: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var(name).ok())
    }
}

/// Instruction budget, so a manifest that never terminates fails instead of hanging.
const BUDGET: u64 = 50_000_000;

/// Heap ceiling for the manifest interpreter.
const MEMORY: usize = 32 * 1024 * 1024;

/// A manifest front end, selected by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Evaluated as a program in a hardened interpreter.
    Luau,
    /// Parsed leniently: comments, trailing commas and unquoted keys are accepted.
    Json,
    /// TOML.
    Toml,
}

impl Format {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "luau" => Some(Self::Luau),
            "json" | "jsonc" | "json5" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

/// A parsed manifest and the paths it was read against.
#[derive(Debug)]
pub struct Loaded {
    pub manifest: Manifest,
    /// Absolute path the manifest was read from.
    pub origin: AbsPath,
    /// Directory relative paths resolve against.
    pub root: AbsPath,
}

/// Finds the manifest in `directory`, then in each of its ancestors.
///
/// Walking up means `pcmp build` works from anywhere inside a project, the way `git`
/// and `cargo` do. The manifest's own directory is still what relative paths resolve
/// against, so where the command was run from changes nothing about the build.
pub fn discover(directory: &AbsPath) -> Result<AbsPath> {
    let mut cursor = Some(directory.clone());

    while let Some(dir) = cursor {
        for candidate in CANDIDATES {
            if let Ok(path) = dir.join(candidate)
                && path.is_file()
            {
                return Ok(path);
            }
        }
        cursor = dir.parent().filter(|parent| *parent != dir);
    }

    Err(Error::NoManifest(
        directory.to_string(),
        CANDIDATES.join(", "),
    ))
}

/// Reads and parses the manifest at `path`, resolving its project root.
pub fn load(path: &AbsPath, env: &Env) -> Result<Loaded> {
    let format = path
        .extension()
        .and_then(Format::from_extension)
        .ok_or_else(|| Error::UnknownFormat(path.to_string()))?;

    let text = std::fs::read_to_string(path.as_std())
        .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;

    let manifest = parse(&text, path.as_str(), format, env)?;

    let dir = path
        .parent()
        .ok_or_else(|| Error::NoManifest(path.to_string(), CANDIDATES.join(", ")))?;

    Ok(Loaded {
        manifest,
        origin: path.clone(),
        root: dir,
    })
}

/// Parses manifest text already in memory. `origin` is used only in messages.
pub fn parse(text: &str, origin: &str, format: Format, env: &Env) -> Result<Manifest> {
    let mut manifest = match format {
        Format::Luau => return eval(text, origin, env),
        Format::Json => json5::from_str(text)
            .map_err(|e| Error::Syntax(origin.into(), "JSON", e.to_string()))?,
        Format::Toml => {
            toml::from_str(text).map_err(|e| Error::Syntax(origin.into(), "TOML", e.to_string()))?
        }
    };

    Manifest::normalise(&mut manifest);
    Ok(manifest)
}

/// Evaluates a Luau manifest in a hardened interpreter.
pub fn eval(source: &str, origin: &str, env: &Env) -> Result<Manifest> {
    let vm = |action: &str, e: mlua::Error| {
        Error::Vm(origin.to_owned(), action.to_owned(), e.to_string())
    };

    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::UTF8,
        LuaOptions::new(),
    )
    .map_err(|e| vm("create the interpreter", e))?;

    // Both must precede `sandbox(true)`, which freezes the global and library tables.
    // Reversed, this fails with "attempt to modify a readonly table".
    harden(&lua, origin)?;
    install_api(&lua, origin, env)?;

    lua.sandbox(true).map_err(|e| vm("enable the sandbox", e))?;
    lua.set_memory_limit(MEMORY)
        .map_err(|e| vm("set the memory limit", e))?;

    let steps = Arc::new(AtomicU64::new(0));
    lua.set_interrupt(move |_| {
        if steps.fetch_add(1, Ordering::Relaxed) > BUDGET {
            return Err(mlua::Error::runtime(
                "manifest exceeded its evaluation budget, so it must terminate",
            ));
        }
        Ok(VmState::Continue)
    });

    let value: Value = lua
        .load(source)
        .set_name(origin)
        .eval()
        .map_err(|e| Error::Eval(origin.into(), e.to_string()))?;

    if !matches!(value, Value::Table(_)) {
        return Err(Error::NotATable(origin.into(), value.type_name().into()));
    }

    let mut manifest: Manifest = lua
        .from_value(value)
        .map_err(|e| Error::Shape(origin.into(), e.to_string()))?;

    manifest.normalise();
    Ok(manifest)
}

fn harden(lua: &Lua, origin: &str) -> Result<()> {
    let globals = lua.globals();
    let vm = |action: String, e: mlua::Error| Error::Vm(origin.to_owned(), action, e.to_string());

    for name in REVOKED {
        globals
            .set(*name, Value::Nil)
            .map_err(|e| vm(format!("revoke `{name}`"), e))?;
    }

    for (library, member) in REVOKED_MEMBERS {
        let table: Option<mlua::Table> = globals
            .get(*library)
            .map_err(|e| vm(format!("read `{library}`"), e))?;

        if let Some(table) = table {
            table
                .set(*member, Value::Nil)
                .map_err(|e| vm(format!("revoke `{library}.{member}`"), e))?;
        }
    }

    Ok(())
}

/// Installs the `pcmp` global, a manifest's only channel outward.
fn install_api(lua: &Lua, origin: &str, env: &Env) -> Result<()> {
    let vm = |action: &str, e: mlua::Error| {
        Error::Vm(origin.to_owned(), action.to_owned(), e.to_string())
    };

    let api = lua.create_table().map_err(|e| vm("create the api", e))?;

    // An error rather than an empty string, so a missing variable cannot become a
    // release stamped with nothing.
    let read = env.clone();
    let required = lua
        .create_function(move |_, name: String| {
            read.get(&name).ok_or_else(|| {
                mlua::Error::runtime(format!(
                    "environment variable `{name}` is not set. \
                     Pass `--env {name}=VALUE`, or use pcmp.envOr(name, fallback)"
                ))
            })
        })
        .map_err(|e| vm("create pcmp.env", e))?;

    // The fallback exists, but the manifest spells it out.
    let read = env.clone();
    let optional = lua
        .create_function(move |_, (name, fallback): (String, String)| {
            Ok(read.get(&name).unwrap_or(fallback))
        })
        .map_err(|e| vm("create pcmp.envOr", e))?;

    api.set("env", required)
        .and_then(|()| api.set("envOr", optional))
        .map_err(|e| vm("populate the api", e))?;

    lua.globals()
        .set("pcmp", api)
        .map_err(|e| vm("install the pcmp global", e))
}

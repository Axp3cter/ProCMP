//! The Luau front end.
//!
//! Luau's sandbox removes `os`, `io`, `load`, `dofile`, `debug` and `coroutine`, and
//! [`REVOKED`] removes the rest. The only channel outward is `pcmp.env`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use mlua::{Lua, LuaOptions, LuaSerdeExt, StdLib, Value, VmState};

use super::Manifest;
use crate::error::{Error, Result};

/// Globals that survive the sandbox but would let a manifest observe the outside, load
/// code at runtime, or return a different value on a second evaluation.
pub const REVOKED: &[&str] = &[
    "collectgarbage",
    "getfenv",
    "loadstring",
    "newproxy",
    "require",
    "setfenv",
];

const REVOKED_MEMBERS: &[(&str, &str)] = &[("math", "random"), ("math", "randomseed")];

/// Bounds a manifest that never terminates.
const BUDGET: u64 = 50_000_000;
const MEMORY: usize = 32 * 1024 * 1024;

/// What `pcmp.env` reads: `--env` values, then the process environment. Passed
/// explicitly rather than exported.
#[derive(Debug, Clone, Default)]
pub struct Env(Vec<(String, String)>);

impl Env {
    /// Last occurrence of a key wins.
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

pub fn eval(source: &str, origin: &str, env: &Env) -> Result<Manifest> {
    let vm =
        |action: &str, e: mlua::Error| Error::Vm(origin.to_owned(), action.into(), e.to_string());

    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::UTF8,
        LuaOptions::new(),
    )
    .map_err(|e| vm("create the interpreter", e))?;

    // Both must precede `sandbox`, which freezes the global and library tables.
    harden(&lua, origin)?;
    install_api(&lua, origin, env)?;

    lua.sandbox(true).map_err(|e| vm("enable the sandbox", e))?;
    lua.set_memory_limit(MEMORY)
        .map_err(|e| vm("set the memory limit", e))?;

    let steps = AtomicU64::new(0);
    lua.set_interrupt(move |_| {
        if steps.fetch_add(1, Ordering::Relaxed) >= BUDGET {
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
        .map_err(|e| Error::Syntax(origin.into(), "Luau", e.to_string()))?;

    manifest.normalise();
    Ok(manifest)
}

fn harden(lua: &Lua, origin: &str) -> Result<()> {
    let globals = lua.globals();
    let vm = |action: String, e: mlua::Error| Error::Vm(origin.to_owned(), action, e.to_string());

    // Luau's own `print` writes to stdout, where it would land in the middle of `--json`.
    let printed = lua
        .create_function(|_, values: mlua::Variadic<Value>| {
            let line = values
                .iter()
                .map(Value::to_string)
                .collect::<mlua::Result<Vec<_>>>()?;
            eprintln!("{}", line.join("\t"));
            Ok(())
        })
        .map_err(|e| vm("redirect `print`".to_owned(), e))?;

    globals
        .set("print", printed)
        .map_err(|e| vm("redirect `print`".to_owned(), e))?;

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

fn install_api(lua: &Lua, origin: &str, env: &Env) -> Result<()> {
    let vm =
        |action: &str, e: mlua::Error| Error::Vm(origin.to_owned(), action.into(), e.to_string());

    let api = lua.create_table().map_err(|e| vm("create the api", e))?;
    let shared = Arc::new(env.clone());

    // An error rather than an empty string.
    let read = Arc::clone(&shared);
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

    let read = Arc::clone(&shared);
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

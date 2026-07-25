//! Finding a manifest and reading it.
//!
//! Format comes from the extension, never from content, so the same bytes cannot mean
//! different things in different directories.

use super::{Env, Manifest};
use crate::error::{Error, Result};
use crate::path::AbsPath;

/// Discovery order. JSON5 leads because it is what `pcmp init` writes: plain data any
/// tool can rewrite, with `$schema` giving an editor validation. Luau comes last and is
/// the only format that can compute a value rather than be given one.
pub const CANDIDATES: &[&str] = &[
    "pcmp.json5",
    "pcmp.json",
    "pcmp.jsonc",
    "pcmp.toml",
    "pcmp.luau",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Luau,
    /// Lenient: comments, trailing commas and unquoted keys are accepted.
    Json,
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

#[derive(Debug)]
pub struct Loaded {
    pub manifest: Manifest,
    pub origin: AbsPath,
    /// Directory relative paths resolve against.
    pub root: AbsPath,
}

/// Searches `directory`, then each ancestor, so `pcmp` works from anywhere inside a
/// project. Relative paths still resolve against the manifest's own directory.
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

pub fn load(path: &AbsPath, env: &Env) -> Result<Loaded> {
    let format = path
        .extension()
        .and_then(Format::from_extension)
        .ok_or_else(|| Error::UnknownFormat(path.to_string()))?;

    let text = std::fs::read_to_string(path.as_std())
        .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;

    let root = path
        .parent()
        .ok_or_else(|| Error::NoManifest(path.to_string(), CANDIDATES.join(", ")))?;

    Ok(Loaded {
        manifest: parse(&text, path.as_str(), format, env)?,
        origin: path.clone(),
        root,
    })
}

/// `origin` is used only in messages.
pub fn parse(text: &str, origin: &str, format: Format, env: &Env) -> Result<Manifest> {
    let mut manifest = match format {
        Format::Luau => return super::luau::eval(text, origin, env),
        Format::Json => json5::from_str(text)
            .map_err(|e| Error::Syntax(origin.into(), "JSON", e.to_string()))?,
        Format::Toml => {
            toml::from_str(text).map_err(|e| Error::Syntax(origin.into(), "TOML", e.to_string()))?
        }
    };

    Manifest::normalise(&mut manifest);
    Ok(manifest)
}

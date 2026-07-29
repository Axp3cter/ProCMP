//! Finding a manifest and reading it.
//!
//! Format comes from the extension, never from content: a JSON manifest called
//! `pcmp.conf` is a mistake worth naming rather than a puzzle worth solving.

use std::rc::Rc;

use super::{Loaders, Manifest, ledger::Reader, luau};
use crate::report::{Code, Diagnostic, Location};
use crate::vfs::{self, AbsPath};

/// Discovery order. JSON5 leads because it is what `pcmp init` writes. Luau comes last
/// and is the only format that can compute a value rather than be given one.
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
    pub fn of(extension: &str) -> Option<Self> {
        match extension {
            "luau" => Some(Self::Luau),
            "json" | "jsonc" | "json5" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Luau => "Luau",
            Self::Json => "JSON",
            Self::Toml => "TOML",
        }
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub manifest: Manifest,
    /// The manifest itself, which `watch` follows even when it sits outside every root.
    pub path: AbsPath,
    /// The directory every relative path resolves against, never the working directory.
    pub root: AbsPath,
}

/// Searches `directory`, then each ancestor.
pub fn discover(directory: &AbsPath) -> Result<AbsPath, Diagnostic> {
    let mut cursor = Some(directory.clone());

    while let Some(here) = cursor {
        for candidate in CANDIDATES {
            if let Ok(path) = here.join(candidate)
                && vfs::is_file(&path)
            {
                return Ok(path);
            }
        }
        cursor = here.parent().filter(|parent| *parent != here);
    }

    Err(Diagnostic::new(
        Code::NoManifest,
        format!("no manifest in `{directory}` or any directory above it"),
    )
    .help(format!("looked for: {}", CANDIDATES.join(", "))))
}

/// The manifest already in `root`, which `init` refuses to write over.
pub fn existing(root: &AbsPath) -> Option<AbsPath> {
    CANDIDATES
        .iter()
        .filter_map(|name| root.join(name).ok())
        .find(vfs::is_file)
}

pub fn load(path: &AbsPath, reader: &Rc<Reader>) -> Result<Loaded, Diagnostic> {
    let format = path.extension().and_then(Format::of).ok_or_else(|| {
        Diagnostic::new(
            Code::UnknownFormat,
            format!("`{path}` has an unsupported extension"),
        )
        .help("supported: json5, json, jsonc, toml, luau")
    })?;

    let root = path.parent().ok_or_else(|| {
        Diagnostic::new(
            Code::NoManifest,
            format!("`{path}` has no parent directory"),
        )
    })?;

    let text = vfs::read_to_string(path)?;

    Ok(Loaded {
        manifest: parse(&text, path.as_str(), format, &root, reader)?,
        path: path.clone(),
        root,
    })
}

/// `origin` names the file in messages, and `root` is what a Luau manifest reads relative to.
pub fn parse(
    text: &str,
    origin: &str,
    format: Format,
    root: &AbsPath,
    reader: &Rc<Reader>,
) -> Result<Manifest, Diagnostic> {
    let manifest = match format {
        Format::Luau => luau::eval(text, origin, root, reader)?,
        Format::Json => json5::from_str(text).map_err(|error| syntax(origin, format, &error))?,
        Format::Toml => toml::from_str(text).map_err(|error| syntax(origin, format, &error))?,
    };

    if format == Format::Luau {
        ordered_loaders(&manifest)?;
    }

    Ok(manifest)
}

/// Covers both a parse failure and a manifest that parses into something that is not one:
/// `json5` and `toml` deserialise in a single call, so the two cannot be told apart, and
/// claiming "not valid JSON" about a file that is valid JSON would be worse than saying
/// less.
fn syntax(origin: &str, format: Format, error: &impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        Code::Syntax,
        format!("`{origin}` is not a valid {} manifest", format.name()),
    )
    .help(error.to_string())
}

/// darklua takes the first matching loader pattern, and a Luau table has no order, so the
/// map spelling would leave hash order deciding which one wins.
fn ordered_loaders(manifest: &Manifest) -> Result<(), Diagnostic> {
    let declared = manifest
        .templates
        .iter()
        .map(|(name, profile)| ("templates", name, profile))
        .chain(
            manifest
                .profiles
                .iter()
                .map(|(name, profile)| ("profiles", name, profile)),
        );

    for (map, name, profile) in declared {
        if matches!(&profile.loaders, Some(Loaders::Map(map)) if !map.is_empty()) {
            return Err(unordered(Location::new(map, name).field("loaders")));
        }

        for (axis, values) in &profile.axes {
            let unordered_overlay = values.values().into_iter().any(|value| {
                values
                    .overlay(value)
                    .is_some_and(|overlay| {
                        matches!(&overlay.loaders, Some(Loaders::Map(map)) if !map.is_empty())
                    })
            });

            if unordered_overlay {
                return Err(unordered(
                    Location::new(map, name).field("axes").field(axis),
                ));
            }
        }
    }

    Ok(())
}

fn unordered(at: Location) -> Diagnostic {
    Diagnostic::new(
        Code::LoadersUnordered,
        format!("`{at}` declares `loaders` as a table"),
    )
    .at(at)
    .help("write a list of `{ pattern = ..., use = ... }`, which has an order")
}

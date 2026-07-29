//! `pcmp init`.
//!
//! The starter manifests are template files rather than `format!` strings. The old design
//! built them in Rust, which meant doubling every brace in every `{token}` and left the
//! result unreadable in the one place it most needs to read like what it becomes. These
//! are the file, with two placeholders.
//!
//! Nothing else is written. A generated `pcmp.schema.json` committed to a repository goes
//! stale on the next upgrade with nothing to notice; `pcmp schema` emits one on demand and
//! `check` reports [`Code::StaleSchema`] when a committed copy has drifted.

use super::format;
use crate::report::{Code, Diagnostic};
use crate::vfs::{self, AbsPath, RelPath};

const JSON5: &str = include_str!("json5.tmpl");
const LUAU: &str = include_str!("luau.tmpl");

/// Where an entry point usually lives, in the order worth trying.
const LIKELY: &[&str] = &["src/init.luau", "src/main.luau", "init.luau", "main.luau"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// Plain data.
    Json5,
    /// Adds `pcmp.env`, `pcmp.now` and `pcmp.read`.
    Luau,
}

impl Format {
    const fn parts(self) -> (&'static str, &'static str) {
        match self {
            Self::Json5 => ("pcmp.json5", JSON5),
            Self::Luau => ("pcmp.luau", LUAU),
        }
    }
}

/// Refuses to write over a manifest that is already here. Only this directory is checked:
/// a project above is not this project.
pub fn write(
    root: &AbsPath,
    name: &str,
    entry: Option<&str>,
    format: Format,
) -> Result<RelPath, Diagnostic> {
    if let Some(existing) = format::existing(root) {
        return Err(Diagnostic::new(
            Code::WriteFailed,
            format!("`{existing}` already exists"),
        ));
    }

    let entry = match entry {
        Some(given) => given.to_owned(),
        None => LIKELY
            .iter()
            .find(|candidate| root.join(**candidate).is_ok_and(|path| vfs::is_file(&path)))
            .map(|found| (*found).to_owned())
            .ok_or_else(|| {
                Diagnostic::new(Code::MissingEntry, "no entry point found")
                    .help(format!("looked for: {}", LIKELY.join(", ")))
                    .help("name one with --entry")
            })?,
    };

    let (file, body) = format.parts();
    let path = root.join(file)?;

    let written = body
        .replace("%NAME%", &escaped(name, format))
        .replace("%ENTRY%", &escaped(&entry, format));

    vfs::write(&path, written.as_bytes())?;
    RelPath::new(file)
}

/// A project name can hold a quote or a backslash.
///
/// JSON writes a control character as `\u` with four digits, which Luau reads as a `u`
/// followed by digits, so the two formats cannot share one escaper.
fn escaped(value: &str, format: Format) -> String {
    match format {
        Format::Json5 => serde_json::Value::String(value.to_owned())
            .to_string()
            .trim_matches('"')
            .to_owned(),
        Format::Luau => value
            .chars()
            .map(|character| match character {
                '\\' => "\\\\".to_owned(),
                '"' => "\\\"".to_owned(),
                control if control.is_control() => format!("\\{:03}", control as u32),
                other => other.to_string(),
            })
            .collect(),
    }
}

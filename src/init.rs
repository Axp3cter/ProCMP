//! Scaffolding a new project.
//!
//! Writes two files and nothing else: a manifest, and the type definitions that make
//! it complete in an editor. No directories, no prompts, no `.gitignore` rewriting.
//! Anything it cannot infer is asked for on the command line rather than guessed.

use crate::error::{Error, Result};
use crate::path::AbsPath;
use crate::{load, schema};

/// Entry points looked for when `--entry` is not given.
const LIKELY_ENTRIES: &[&str] = &["src/init.luau", "src/main.luau", "init.luau", "main.luau"];

/// Files written by `pcmp init`.
#[derive(Debug)]
pub struct Created {
    pub manifest: AbsPath,
    pub definitions: AbsPath,
}

/// Writes a starter manifest and type definitions into `root`.
///
/// # Errors
///
/// When a manifest already exists, or no entry point was given and none could be
/// found. Refusing rather than overwriting keeps this safe to run twice.
pub fn run(root: &AbsPath, name: &str, entry: Option<&str>) -> Result<Created> {
    if let Ok(existing) = load::discover(root) {
        return Err(Error::AlreadyExists(existing.relative_to(root)));
    }

    let entry = match entry {
        Some(given) => given.to_owned(),
        None => LIKELY_ENTRIES
            .iter()
            .find(|candidate| root.join(candidate).is_ok_and(|p| p.is_file()))
            .map(|found| (*found).to_owned())
            .ok_or_else(|| Error::NoEntry(LIKELY_ENTRIES.join(", ")))?,
    };

    let manifest = root.join("pcmp.luau")?;
    std::fs::write(manifest.as_std(), template(name, &entry))
        .map_err(|e| Error::Write(manifest.to_string(), e.to_string()))?;

    let definitions = root.join("pcmp.d.luau")?;
    std::fs::write(definitions.as_std(), schema::luau())
        .map_err(|e| Error::Write(definitions.to_string(), e.to_string()))?;

    Ok(Created {
        manifest,
        definitions,
    })
}

/// Escapes a value for a Luau string literal.
///
/// A project name comes from a directory name or the command line, so it can contain a
/// quote or a backslash. Interpolating it raw produces a manifest that does not parse.
fn escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            '\n' | '\r' | '\t' => vec![' '],
            other => vec![other],
        })
        .collect()
}

/// A manifest with a dev and a release profile, which is the shape almost every
/// project starts from.
fn template(name: &str, entry: &str) -> String {
    let escaped = escape(name);
    let entry = escape(entry);
    format!(
        r#"--!strict
return {{
	vars = {{
		name    = "{escaped}",
		version = pcmp.envOr("VERSION", "v0.0.0-dev"),
	}},

	profiles = {{
		base = {{
			abstract = true,
			entry    = "{entry}",
			output   = "dist/{{profile}}/{{name}}.luau",
			darklua  = {{
				bundle = {{ require_mode = "luau" }},
			}},
		}},

		dev = {{
			extends = "base",
			define  = {{ DEBUG = true }},
			darklua = {{
				generator = "readable",
				rules     = {{ "compute_expression" }},
			}},
		}},

		release = {{
			extends = "base",
			define  = {{ DEBUG = false }},
			header  = {{ "-- {{name}} {{version}}" }},
			darklua = {{
				generator = "dense",
				rules     = {{
					"compute_expression",
					"remove_unused_if_branch",
					"remove_types",
					"remove_comments",
					"rename_variables",
				}},
			}},
		}},
	}},
}}
"#
    )
}

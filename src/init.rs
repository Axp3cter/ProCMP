//! Scaffolding a new project.
//!
//! Writes two files and nothing else: a manifest, and the schema that makes it complete
//! in an editor. No directories, no prompts, no `.gitignore` rewriting. Anything it
//! cannot infer is asked for on the command line rather than guessed.

use crate::error::{Error, Result};
use crate::path::AbsPath;
use crate::{load, schema};

/// Entry points looked for when `--entry` is not given.
const LIKELY_ENTRIES: &[&str] = &["src/init.luau", "src/main.luau", "init.luau", "main.luau"];

/// Which manifest a fresh project starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// JSON5, paired with a JSON Schema. Comments and trailing commas are accepted, and
    /// `$schema` gives every editor validation with no further setup.
    Json5,
    /// Luau, paired with type definitions. Adds `pcmp.env` for values a manifest has to
    /// compute rather than be given.
    Luau,
}

/// Files written by `pcmp init`.
#[derive(Debug)]
pub struct Created {
    /// The manifest that was written.
    pub manifest: AbsPath,
    /// The schema or type definitions written beside it.
    pub definitions: AbsPath,
}

/// Writes a starter manifest and its schema into `root`.
///
/// # Errors
///
/// When a manifest already exists, or no entry point was given and none could be
/// found. Refusing rather than overwriting keeps this safe to run twice.
pub fn run(root: &AbsPath, name: &str, entry: Option<&str>, format: Format) -> Result<Created> {
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

    let (manifest_name, definitions_name, body, definitions) = match format {
        Format::Json5 => (
            "pcmp.json5",
            "pcmp.schema.json",
            json5(name, &entry),
            schema::json(),
        ),
        Format::Luau => (
            "pcmp.luau",
            "pcmp.d.luau",
            luau(name, &entry),
            schema::luau(),
        ),
    };

    // Both paths are resolved and both files checked before either is written. Writing
    // the manifest and then failing would leave a project that `init` refuses to
    // scaffold again and that `pcmp plan` cannot resolve.
    let manifest_path = free(root, manifest_name)?;
    let definitions_path = free(root, definitions_name)?;

    write(&manifest_path, &body)?;
    write(&definitions_path, &definitions)?;

    Ok(Created {
        manifest: manifest_path,
        definitions: definitions_path,
    })
}

/// Resolves `name` under `root`, refusing a path that already holds something.
fn free(root: &AbsPath, name: &str) -> Result<AbsPath> {
    let path = root.join(name)?;

    if path.exists() {
        return Err(Error::AlreadyExists(path.relative_to(root)));
    }

    Ok(path)
}

fn write(path: &AbsPath, body: &str) -> Result<()> {
    std::fs::write(path.as_std(), body).map_err(|e| Error::Write(path.to_string(), e.to_string()))
}

/// Quotes a value as a JSON string.
///
/// A project name comes from a directory name or the command line, so it can contain a
/// quote or a backslash. Interpolating it raw produces a manifest that does not parse.
fn quote(value: &str) -> String {
    serde_json::Value::String(value.to_owned()).to_string()
}

/// Quotes a value as a Luau string.
///
/// Not [`quote`]: JSON escapes a control character as `\u0000`, which Luau reads as a
/// `u` followed by four digits. Luau spells the same thing `\u{0}`, and anything below
/// a space is rare enough in a project name to replace with one.
fn quote_luau(value: &str) -> String {
    let escaped: String = value
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".to_owned(),
            '"' => "\\\"".to_owned(),
            c if c.is_control() => " ".to_owned(),
            c => c.to_string(),
        })
        .collect();

    format!("\"{escaped}\"")
}

/// A dev and a release profile, which is the shape almost every project starts from.
fn json5(name: &str, entry: &str) -> String {
    let name = quote(name);
    let entry = quote(entry);
    format!(
        r#"{{
  $schema: "./pcmp.schema.json",

  // Each becomes a {{token}} in output and header, and a PCMP_<NAME> constant.
  // Override per build with `pcmp build --var version=v1.2.3`.
  vars: {{
    name: {name},
    version: "v0.0.0-dev",
  }},

  profiles: {{
    base: {{
      abstract: true,
      entry: {entry},
      output: "dist/{{profile}}/{{name}}.luau",
      darklua: {{
        bundle: {{ require_mode: "luau" }},
      }},
    }},

    dev: {{
      extends: "base",
      define: {{ DEBUG: true }},
      darklua: {{
        generator: "readable",
        rules: ["compute_expression"],
      }},
    }},

    release: {{
      extends: "base",
      define: {{ DEBUG: false }},
      header: ["-- {{name}} {{version}}"],
      darklua: {{
        generator: "dense",
        rules: [
          "compute_expression",
          "remove_unused_if_branch",
          "remove_types",
          "remove_comments",
          "rename_variables",
        ],
      }},
    }},
  }},
}}
"#
    )
}

/// The same project as [`json5`], with `pcmp.envOr` in place of a literal version.
fn luau(name: &str, entry: &str) -> String {
    let name = quote_luau(name);
    let entry = quote_luau(entry);
    format!(
        r#"--!strict
return {{
	vars = {{
		name    = {name},
		version = pcmp.envOr("VERSION", "v0.0.0-dev"),
	}},

	profiles = {{
		base = {{
			abstract = true,
			entry    = {entry},
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

//! Scaffolding a new project: a manifest and its schema, nothing else.

use crate::error::{Error, Result};
use crate::path::AbsPath;
use crate::{manifest, schema};

const LIKELY_ENTRIES: &[&str] = &["src/init.luau", "src/main.luau", "init.luau", "main.luau"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// Plain data, paired with a JSON Schema.
    Json5,
    /// Adds `pcmp.env`, paired with type definitions.
    Luau,
}

#[derive(Debug)]
pub struct Created {
    pub manifest: AbsPath,
    pub definitions: AbsPath,
}

pub fn run(root: &AbsPath, name: &str, entry: Option<&str>, format: Format) -> Result<Created> {
    if let Ok(existing) = manifest::discover(root) {
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

    let (names, body, definitions) = match format {
        Format::Json5 => (
            ("pcmp.json5", "pcmp.schema.json"),
            json5(name, &entry),
            schema::json(),
        ),
        Format::Luau => (
            ("pcmp.luau", "pcmp.d.luau"),
            luau(name, &entry),
            schema::luau(),
        ),
    };

    // Both paths checked before either is written.
    let manifest = free(root, names.0)?;
    let definitions_path = free(root, names.1)?;

    write(&manifest, &body)?;
    write(&definitions_path, &definitions)?;

    Ok(Created {
        manifest,
        definitions: definitions_path,
    })
}

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

/// A name can hold a quote or a backslash.
fn quote(value: &str) -> String {
    serde_json::Value::String(value.to_owned()).to_string()
}

/// Not [`quote`]: JSON writes a control character as a `\u` escape with four digits,
/// which Luau reads as a `u` followed by digits.
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

fn json5(name: &str, entry: &str) -> String {
    let name = quote(name);
    let entry = quote(entry);
    format!(
        r#"{{
  $schema: "./pcmp.schema.json",

  // Each becomes a {{token}} and a PCMP_<NAME> constant.
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
			darklua  = {{ bundle = {{ require_mode = "luau" }} }},
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

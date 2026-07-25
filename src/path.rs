//! Absolute, lexically normalised, UTF-8 paths.
//!
//! Lexical means no symlink resolution and no existence check.

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

use crate::error::{Error, Result};

/// Absoluteness and `..` traversal are checked once, at construction.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(transparent)]
pub struct AbsPath(Utf8PathBuf);

impl AbsPath {
    pub fn new(path: impl AsRef<Utf8Path>) -> Result<Self> {
        let path = path.as_ref();

        if path.as_str().is_empty() {
            return Err(Error::EmptyPath);
        }
        if !path.is_absolute() {
            return Err(Error::NotAbsolute(path.to_string()));
        }

        Ok(Self(normalise(path)?))
    }

    /// Resolves against this directory, leaving absolute inputs unchanged.
    pub fn join(&self, path: impl AsRef<Utf8Path>) -> Result<Self> {
        let path = path.as_ref();

        if path.as_str().is_empty() {
            return Err(Error::EmptyPath);
        }

        let joined = if path.is_absolute() {
            path.to_owned()
        } else {
            self.0.join(path)
        };

        Ok(Self(normalise(&joined)?))
    }

    /// The only place the crate reads the process working directory.
    pub fn cwd() -> Result<Self> {
        let cwd = std::env::current_dir().map_err(|e| Error::Cwd(e.to_string()))?;
        let cwd = Utf8PathBuf::from_path_buf(cwd)
            .map_err(|p| Error::Cwd(format!("`{}` is not UTF-8", p.display())))?;
        Self::new(cwd)
    }

    pub fn parent(&self) -> Option<Self> {
        self.0.parent().map(|p| Self(p.to_owned()))
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    pub fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    /// Falls back to the absolute form when not under `base`, and renders an equal path
    /// as `.`.
    pub fn relative_to(&self, base: &AbsPath) -> String {
        match self.0.strip_prefix(&base.0) {
            Ok(relative) if relative.as_str().is_empty() => ".".to_owned(),
            Ok(relative) => relative.to_string(),
            Err(_) => self.0.to_string(),
        }
    }

    pub fn as_std(&self) -> &std::path::Path {
        self.0.as_std_path()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn is_file(&self) -> bool {
        self.0.as_std_path().is_file()
    }

    pub fn exists(&self) -> bool {
        self.0.as_std_path().exists()
    }

    /// Component-wise, so `/a/bc` is not under `/a/b`.
    pub fn is_under(&self, base: &AbsPath) -> bool {
        self.0.starts_with(&base.0)
    }

    pub fn is_within(&self, base: &AbsPath) -> bool {
        self != base && self.is_under(base)
    }
}

fn normalise(path: &Utf8Path) -> Result<Utf8PathBuf> {
    let mut out = Utf8PathBuf::new();

    for component in path.components() {
        match component {
            Utf8Component::Prefix(prefix) => out.push(prefix.as_str()),
            Utf8Component::RootDir => out.push("/"),
            Utf8Component::CurDir => {}
            // Clamping silently would let `../../..` escape the project.
            Utf8Component::ParentDir => {
                if !out.pop() {
                    return Err(Error::EscapesRoot(path.to_string()));
                }
            }
            Utf8Component::Normal(segment) => out.push(segment),
        }
    }

    Ok(out)
}

impl std::fmt::Display for AbsPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

//! Absolute, lexically normalised, UTF-8 paths.
//!
//! Normalisation is lexical: no symlink resolution, no existence check. That keeps
//! resolution pure and lets a plan be built without touching disk.

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

use crate::error::{Error, Result};

/// Constructing one is the only place absoluteness and `..` traversal are checked, so
/// holding an `AbsPath` is proof both already passed.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(transparent)]
pub struct AbsPath(Utf8PathBuf);

impl AbsPath {
    /// Wraps a path that is already absolute.
    ///
    /// # Errors
    ///
    /// When the path is empty, relative, or resolves above the filesystem root.
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

    /// Resolves `path` against this directory, leaving absolute inputs unchanged.
    ///
    /// # Errors
    ///
    /// When `path` is empty or the result resolves above the filesystem root.
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

    /// The only place the crate reads the process working directory. Everywhere else
    /// takes a root explicitly.
    ///
    /// # Errors
    ///
    /// When the working directory is unreadable or is not UTF-8.
    pub fn cwd() -> Result<Self> {
        let cwd = std::env::current_dir().map_err(|e| Error::Cwd(e.to_string()))?;
        let cwd = Utf8PathBuf::from_path_buf(cwd)
            .map_err(|p| Error::Cwd(format!("`{}` is not UTF-8", p.display())))?;
        Self::new(cwd)
    }

    /// The containing directory, or [`None`] at the filesystem root.
    pub fn parent(&self) -> Option<Self> {
        self.0.parent().map(|p| Self(p.to_owned()))
    }

    /// The final component, or [`None`] at the filesystem root.
    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// The extension of the final component, without its dot.
    pub fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    /// Renders relative to `base`, falling back to the absolute form when not under it.
    ///
    /// This feeds diagnostics, where a longer path beats a missing message, and hasher
    /// labels, where an empty one would be indistinguishable from an absent field. A
    /// path equal to its base renders as `.` rather than as nothing.
    pub fn relative_to(&self, base: &AbsPath) -> String {
        match self.0.strip_prefix(&base.0) {
            Ok(relative) if relative.as_str().is_empty() => ".".to_owned(),
            Ok(relative) => relative.to_string(),
            Err(_) => self.0.to_string(),
        }
    }

    /// Borrows as a [`std::path::Path`], for `std::fs` and darklua.
    pub fn as_std(&self) -> &std::path::Path {
        self.0.as_std_path()
    }

    /// Borrows as a string, which is always valid UTF-8 here.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Whether a regular file exists at this path. Touches disk.
    pub fn is_file(&self) -> bool {
        self.0.as_std_path().is_file()
    }

    /// Whether a directory exists at this path. Touches disk.
    pub fn is_dir(&self) -> bool {
        self.0.as_std_path().is_dir()
    }

    /// Whether anything exists at this path. Touches disk.
    pub fn exists(&self) -> bool {
        self.0.as_std_path().exists()
    }

    /// Whether this path is `base` or sits inside it. Lexical, like everything else.
    ///
    /// Component-wise rather than textual, so `/a/bc` is not under `/a/b`.
    pub fn is_under(&self, base: &AbsPath) -> bool {
        self.0.starts_with(&base.0)
    }

    /// Whether this path sits strictly inside `base`.
    pub fn is_within(&self, base: &AbsPath) -> bool {
        self != base && self.is_under(base)
    }
}

/// Drops `.`, resolves `..`, and collapses separators.
fn normalise(path: &Utf8Path) -> Result<Utf8PathBuf> {
    let mut out = Utf8PathBuf::new();

    for component in path.components() {
        match component {
            Utf8Component::Prefix(prefix) => out.push(prefix.as_str()),
            Utf8Component::RootDir => out.push("/"),
            Utf8Component::CurDir => {}
            // Refusing to normalise past the root is a correctness check, not a
            // fallback: clamping silently would let `../../..` escape the project.
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

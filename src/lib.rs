//! Multi-target build composition for Luau projects.
//!
//! darklua is linked in rather than shelled out to. Manifests are evaluated with every
//! entropy source revoked, so config-as-code stays reproducible.
//!
//! Headless: rendering and argument parsing live in the `pcmp` binary.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod build;
pub mod check;
pub mod diag;
pub mod digest;
pub mod error;
pub mod init;
pub mod manifest;
pub mod path;
pub mod plan;
pub mod schema;

pub use build::{Engine, Outcome, Report, Scope};
pub use diag::{Diag, Severity};
pub use digest::{Digest, Hasher};
pub use error::{Error, ExitCode, Result};
pub use manifest::Manifest;
pub use path::AbsPath;
pub use plan::{Graph, Overrides, Task};

/// The darklua this binary is linked against, baked in from the lockfile.
pub fn darklua_version() -> &'static str {
    env!("DARKLUA_VERSION")
}

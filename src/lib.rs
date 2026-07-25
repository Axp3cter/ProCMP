//! Multi-target build composition for Luau projects.
//!
//! darklua is linked in as a library rather than shelled out to, so there is no
//! `PATH` lookup, no version drift, and no temp files. Manifests are evaluated in a
//! Luau VM with every entropy source revoked, so config-as-code stays reproducible.
//!
//! Everything here is headless. Rendering, argument parsing and the `--pick` menu live
//! in the `pcmp` binary, so nothing in this crate can decide to write to a terminal.

#![forbid(unsafe_code)]
#![warn(clippy::all, missing_docs)]

pub mod diag;
pub mod digest;
pub mod engine;
pub mod error;
pub mod init;
pub mod lint;
pub mod load;
pub mod manifest;
pub mod path;
pub mod plan;
pub mod rules;
pub mod schema;
pub mod watch;

pub use diag::{Diag, Severity};
pub use digest::{Digest, Hasher};
pub use engine::{Engine, Outcome, Report};
pub use error::{Error, ExitCode, Result};
pub use manifest::Manifest;
pub use path::AbsPath;
pub use plan::{Graph, Overrides, Task};

/// Returns the darklua version this binary is linked against.
///
/// Baked in at compile time from the lockfile, so it cannot disagree with the code
/// actually running.
pub fn darklua_version() -> &'static str {
    env!("DARKLUA_VERSION")
}

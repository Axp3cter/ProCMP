//! Multi-target build composition for Luau projects.
//!
//! darklua is linked in as a library rather than shelled out to, so there is no
//! `PATH` lookup, no version drift, and no temp files. Manifests are evaluated in a
//! Luau VM with every entropy source revoked, so config-as-code stays reproducible.
//!
//! # Conventions
//!
//! Missing data is reported, never replaced with a placeholder. A default appears only
//! where the reason for it appears beside it. Parsing happens once, at the manifest
//! boundary, and nothing downstream re-validates. Anything a user sees is ordered, and
//! identical input produces identical bytes. `process::exit` appears only in `main`
//!
//! Comments carry what the code and the names cannot: a choice whose alternative looks
//! obvious, or a change that would break something non-local. Everything else is left
//! to the signature.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

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
pub use plan::{Graph, Task};

/// Returns the darklua version this binary is linked against.
///
/// Baked in at compile time from the lockfile, so it cannot disagree with the code
/// actually running.
pub fn darklua_version() -> &'static str {
    env!("DARKLUA_VERSION")
}

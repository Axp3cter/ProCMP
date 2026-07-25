//! Multi-target build composition for Luau projects.
//!
//! Links in darklua as a library. Rendering and argument parsing live in the `pcmp`
//! binary.

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

/// The darklua this crate links against. Bump with the `=` requirement in `Cargo.toml`,
/// which is what decides the version and cannot resolve to another.
pub const DARKLUA_VERSION: &str = "0.19.0";

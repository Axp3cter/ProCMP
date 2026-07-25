//! Bakes the linked darklua version into the binary.
//!
//! The version reported and mixed into every cache key is read from the lockfile that
//! produced the build, so it cannot disagree with the code actually running.

use std::path::{Path, PathBuf};

fn main() {
    let lock = lockfile().expect("a build always has a Cargo.lock above it");

    // Registered against the path that was actually read. Registering a relative name
    // would leave a workspace build watching a file that is not there.
    println!("cargo:rerun-if-changed={}", lock.display());
    println!("cargo:rerun-if-changed=build.rs");

    let text = std::fs::read_to_string(&lock).expect("Cargo.lock is readable");

    // No fallback: a placeholder version would silently make every cache key wrong and
    // `pcmp --version` misleading. Failing the build is the recoverable outcome.
    let version = text
        .split("[[package]]")
        .find(|block| block.contains("name = \"darklua\""))
        .and_then(|block| block.lines().find(|line| line.starts_with("version = ")))
        .map(|line| line.trim_start_matches("version = ").trim_matches('"'))
        .filter(|version| !version.is_empty())
        .expect("Cargo.lock always records a version for darklua");

    println!("cargo:rustc-env=DARKLUA_VERSION={version}");
}

/// Finds `Cargo.lock` beside the crate or in an ancestor.
///
/// A workspace keeps one lockfile at its root, and `CARGO_MANIFEST_DIR` points at the
/// member. Reading a relative `Cargo.lock` therefore works only for a standalone build.
fn lockfile() -> Option<PathBuf> {
    let start = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut directory: Option<&Path> = Some(Path::new(&start));

    while let Some(current) = directory {
        let candidate = current.join("Cargo.lock");
        if candidate.is_file() {
            return Some(candidate);
        }
        directory = current.parent();
    }

    None
}

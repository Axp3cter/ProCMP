//! Bakes the linked darklua version into the binary, so it cannot disagree with the
//! version mixed into cache keys.

use std::path::{Path, PathBuf};

fn main() {
    let lock = lockfile().expect("Cargo.lock sits at or above the crate");
    println!("cargo:rerun-if-changed={}", lock.display());
    println!("cargo:rerun-if-changed=build.rs");

    let text = std::fs::read_to_string(&lock).expect("Cargo.lock is readable");
    let version = text
        .split("[[package]]")
        .find(|block| block.contains("name = \"darklua\""))
        .and_then(|block| block.lines().find(|line| line.starts_with("version = ")))
        .map(|line| line.trim_start_matches("version = ").trim_matches('"'))
        .filter(|version| !version.is_empty())
        .expect("Cargo.lock records a darklua version");

    println!("cargo:rustc-env=DARKLUA_VERSION={version}");
}

/// A workspace keeps one lockfile at its root, so a relative name is not enough.
fn lockfile() -> Option<PathBuf> {
    let start = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut directory = Some(Path::new(&start));

    while let Some(current) = directory {
        let candidate = current.join("Cargo.lock");
        if candidate.is_file() {
            return Some(candidate);
        }
        directory = current.parent();
    }
    None
}

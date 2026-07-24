//! Bakes the linked darklua version into the binary.
//!
//! The version reported and mixed into every cache key is read from the lockfile that
//! produced the build, so it cannot disagree with the code actually running.

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");

    let lock = std::fs::read_to_string("Cargo.lock").expect("Cargo.lock sits beside this script");

    // No fallback: a placeholder version would silently make every cache key wrong and
    // `pcmp --version` misleading. Failing the build is the recoverable outcome.
    let version = lock
        .split("[[package]]")
        .find(|block| block.contains("name = \"darklua\""))
        .and_then(|block| block.lines().find(|line| line.starts_with("version = ")))
        .map(|line| line.trim_start_matches("version = ").trim_matches('"'))
        .expect("Cargo.lock always records a version for darklua");

    println!("cargo:rustc-env=DARKLUA_VERSION={version}");
}

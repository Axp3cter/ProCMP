//! The crate's error type.
//!
//! Every variant names the file or task it came from and, where a fix exists, states
//! it. A message that says only "invalid config" leaves the reader to find the key
//! themselves.

use thiserror::Error;

/// [`std::result::Result`] specialised to [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Anything that can go wrong between reading a manifest and writing an artifact.
#[derive(Debug, Error)]
pub enum Error {
    #[error("path `{0}` is not absolute")]
    NotAbsolute(String),

    #[error("path `{0}` escapes the filesystem root: too many `..` segments")]
    EscapesRoot(String),

    #[error("path is empty: remove the field or give it a value")]
    EmptyPath,

    #[error("the working directory is not usable: {0}")]
    Cwd(String),

    #[error("no manifest in `{0}`\n  expected one of: {1}\n  run `pcmp schema` to see the format")]
    NoManifest(String, String),

    #[error("`{0}` has an unsupported extension\n  supported: luau, json, jsonc, json5, toml")]
    UnknownFormat(String),

    #[error("could not read `{0}`: {1}")]
    Read(String, String),

    #[error("could not write `{0}`: {1}")]
    Write(String, String),

    #[error("`{0}` is not valid {1}\n  {2}")]
    Syntax(String, &'static str, String),

    #[error(
        "`{0}` does not match the manifest schema\n  {1}\n  run `pcmp schema` for the full shape"
    )]
    Shape(String, String),

    #[error("`{0}` must end with `return {{ ... }}`, but returned {1}")]
    NotATable(String, String),

    #[error("`{0}` failed while evaluating\n  {1}")]
    Eval(String, String),

    #[error("could not {1} for `{0}`\n  {2}")]
    Vm(String, String, String),

    #[error("unknown token `{{{0}}}`\n  known tokens: {1}")]
    UnknownToken(String, String),

    #[error("token `{{{0}}}` has no value\n  set it explicitly on the profile")]
    EmptyToken(String),

    #[error("unterminated `{{` in `{0}`\n  write `{{{{` for a literal brace")]
    UnterminatedToken(String),

    #[error("task `{0}` emitted a darklua configuration darklua rejected\n  {1}\n{2}")]
    DarkluaConfig(String, String, String),

    #[error("task `{0}`: `entry` path `{1}` does not exist")]
    MissingEntry(String, String),

    #[error("task `{0}` failed\n  {1}")]
    Process(String, String),

    #[error("task `{0}` reported no failure but wrote nothing to `{1}`{2}")]
    NoOutput(String, String, &'static str),

    #[error("tasks `{1}` and `{2}` both write to `{0}`\n  give them distinct `output` templates")]
    OutputCollision(String, String, String),

    #[error("no task matched {0}\n  known tasks: {1}")]
    NoSuchTask(String, String),

    #[error("manifest `{0}` could not be resolved")]
    Unresolved(String),

    #[error("could not watch `{0}`: {1}")]
    Watch(String, String),

    #[error("`{0}` is not `KEY=VALUE`")]
    BadPair(String),

    #[error("`{0}` is not a valid glob\n  {1}")]
    BadGlob(String, String),

    #[error("`{0}` already exists")]
    AlreadyExists(String),

    #[error("no entry point found\n  looked for: {0}\n  pass one with --entry")]
    NoEntry(String),
}

/// Process exit codes.
///
/// Distinct per failure class so a CI job can branch on why it failed rather than
/// matching on stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Everything succeeded.
    Success = 0,
    /// A build task failed, or output was not reproducible.
    Build = 1,
    /// The manifest could not be loaded or resolved.
    Config = 2,
    /// Linting reported an error, or a warning under `--strict`.
    Lint = 5,
}

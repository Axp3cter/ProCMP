//! The crate's error type.
//!
//! Every variant leads with the file, path or task it concerns. Detail and the fix
//! follow on indented lines.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("path `{0}` is not absolute")]
    NotAbsolute(String),

    #[error("path `{0}` escapes the filesystem root\n  too many `..` segments")]
    EscapesRoot(String),

    #[error("a path is empty\n  remove the field, or give it a value")]
    EmptyPath,

    #[error("the working directory is not usable\n  {0}")]
    Cwd(String),

    #[error("no manifest in `{0}` or any parent\n  expected one of: {1}")]
    NoManifest(String, String),

    #[error("`{0}` has an unsupported extension\n  supported: json5, json, jsonc, toml, luau")]
    UnknownFormat(String),

    #[error("could not read `{0}`\n  {1}")]
    Read(String, String),

    #[error("could not write `{0}`\n  {1}")]
    Write(String, String),

    #[error("`{0}` is not valid {1}\n  {2}")]
    Syntax(String, &'static str, String),

    #[error("`{0}` must return a table, but returned {1}")]
    NotATable(String, String),

    #[error("`{0}` failed while evaluating\n  {1}")]
    Eval(String, String),

    #[error("`{0}` could not {1}\n  {2}")]
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

    #[error(
        "task `{0}` reported no failure but wrote nothing to `{1}`\n  \
         a file filter that matches nothing is the usual cause: `apply_to_files` and \
         `skip_files` match each file's path relative to the entry, so `src/**` matches \
         nothing when the entry is `src/init.luau`"
    )]
    NoOutput(String, String),

    #[error("tasks `{0}` and `{1}` both write to `{2}`\n  give them distinct `output` templates")]
    OutputCollision(String, String, String),

    #[error("no task matched `{0}`\n  known tasks: {1}")]
    NoSuchTask(String, String),

    #[error("`explain` needs a task\n  name one, or pass `--pick`\n  known tasks: {0}")]
    NoTaskGiven(String),

    #[error("`--pick` needs an interactive terminal\n  name the tasks instead")]
    NotATerminal,

    #[error("the terminal could not be driven\n  {0}")]
    Terminal(String),

    #[error("manifest `{0}` could not be resolved")]
    Unresolved(String),

    #[error("could not watch `{0}`\n  {1}")]
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

/// Distinct per failure class, so CI can branch on why rather than on stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    /// A task failed, or output was not reproducible.
    Build = 1,
    /// The manifest could not be loaded or resolved.
    Config = 2,
    /// Linting failed.
    Lint = 5,
}

//! The crate's error type.
//!
//! Every variant names the file or task it came from and, where a fix exists, states
//! it. A message that says only "invalid config" leaves the reader to find the key
//! themselves.

use thiserror::Error;

/// [`std::result::Result`] specialised to [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Anything that can go wrong between reading a manifest and writing an artifact.
///
/// Every variant leads with the file, path or task it concerns, so a message reads the
/// same way wherever it surfaces. Detail and the fix follow on indented lines.
#[derive(Debug, Error)]
pub enum Error {
    #[error("path `{0}` is not absolute")]
    /// A path that has to be absolute is not.
    NotAbsolute(String),

    #[error("path `{0}` escapes the filesystem root\n  too many `..` segments")]
    /// A path resolves above the filesystem root.
    EscapesRoot(String),

    #[error("a path is empty\n  remove the field, or give it a value")]
    /// A path field was declared with no value.
    EmptyPath,

    #[error("the working directory is not usable\n  {0}")]
    /// The process working directory could not be used.
    Cwd(String),

    #[error("no manifest in `{0}`\n  expected one of: {1}\n  run `pcmp schema` to see the format")]
    /// No manifest was found in a directory or any ancestor.
    NoManifest(String, String),

    #[error("`{0}` has an unsupported extension\n  supported: luau, json, jsonc, json5, toml")]
    /// A manifest extension names no supported format.
    UnknownFormat(String),

    #[error("could not read `{0}`\n  {1}")]
    /// A file or directory could not be read.
    Read(String, String),

    #[error("could not write `{0}`\n  {1}")]
    /// A file could not be written.
    Write(String, String),

    #[error("`{0}` is not valid {1}\n  {2}")]
    /// A manifest is not valid in the format its extension names.
    Syntax(String, &'static str, String),

    #[error(
        "`{0}` does not match the manifest schema\n  {1}\n  run `pcmp schema` for the full shape"
    )]
    /// A manifest parses but does not match the schema.
    Shape(String, String),

    #[error("`{0}` must return a table, but returned {1}")]
    /// A Luau manifest returned something other than a table.
    NotATable(String, String),

    #[error("`{0}` failed while evaluating\n  {1}")]
    /// A Luau manifest failed while running.
    Eval(String, String),

    #[error("`{0}` could not {1}\n  {2}")]
    /// The manifest interpreter could not be set up.
    Vm(String, String, String),

    #[error("unknown token `{{{0}}}`\n  known tokens: {1}")]
    /// A template names a token that does not exist.
    UnknownToken(String, String),

    #[error("token `{{{0}}}` has no value\n  set it explicitly on the profile")]
    /// A template names a token whose value is empty.
    EmptyToken(String),

    #[error("unterminated `{{` in `{0}`\n  write `{{{{` for a literal brace")]
    /// A template opens a token it never closes.
    UnterminatedToken(String),

    #[error("task `{0}` emitted a darklua configuration darklua rejected\n  {1}\n{2}")]
    /// darklua rejected the configuration a task compiled to.
    DarkluaConfig(String, String, String),

    #[error("task `{0}`: `entry` path `{1}` does not exist")]
    /// A task's entry path does not exist.
    MissingEntry(String, String),

    #[error("task `{0}` failed\n  {1}")]
    /// darklua failed while processing a task.
    Process(String, String),

    #[error(
        "task `{0}` reported no failure but wrote nothing to `{1}`\n  \
         a file filter that matches nothing is the usual cause: `apply_to_files` and \
         `skip_files` are matched against each file's path relative to the entry, so \
         `src/**` matches nothing when the entry is `src/init.luau`"
    )]
    /// darklua reported success but wrote nothing.
    NoOutput(String, String),

    #[error("tasks `{0}` and `{1}` both write to `{2}`\n  give them distinct `output` templates")]
    /// Two tasks claim the same output path.
    OutputCollision(String, String, String),

    #[error("no task matched `{0}`\n  known tasks: {1}")]
    /// A selector matched no task.
    NoSuchTask(String, String),

    #[error("`explain` needs a task\n  name one, or pass `--pick`\n  known tasks: {0}")]
    /// `explain` was run with neither a task nor `--pick`.
    NoTaskGiven(String),

    #[error("`--pick` needs an interactive terminal\n  name the tasks instead")]
    /// `--pick` was given where no menu can be drawn.
    NotATerminal,

    #[error("the terminal could not be driven\n  {0}")]
    /// The terminal could not be put into or out of raw mode.
    Terminal(String),

    #[error("manifest `{0}` could not be resolved")]
    /// A manifest loaded but produced no usable plan.
    Unresolved(String),

    #[error("could not watch `{0}`\n  {1}")]
    /// A directory could not be watched.
    Watch(String, String),

    #[error("`{0}` is not `KEY=VALUE`")]
    /// A `KEY=VALUE` argument has no `=`.
    BadPair(String),

    #[error("`{0}` is not a valid glob\n  {1}")]
    /// An `ignore` entry is not a valid glob.
    BadGlob(String, String),

    #[error("`{0}` already exists")]
    /// `init` would overwrite a file that is already there.
    AlreadyExists(String),

    #[error("no entry point found\n  looked for: {0}\n  pass one with --entry")]
    /// `init` found no entry point and was given none.
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

//! The diagnostic catalogue.
//!
//! This is the reference: `pcmp explain <CODE>` prints [`Code::description`], so there is
//! no generated table anywhere that could disagree with it.

use super::{Exit, Severity};

/// Every way a run can go wrong, in the order the phases run.
///
/// A code names one failure and never another, so a message may be reworded but a code
/// may not be reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
    // ── manifest ──────────────────────────────────────────────────────────────────
    NoManifest,
    UnknownFormat,
    Unreadable,
    Syntax,
    NotATable,
    Eval,
    Budget,
    UnsetEnv,

    // ── resolution ────────────────────────────────────────────────────────────────
    UnknownBase,
    CyclicExtends,
    NameCollision,
    BadName,
    MissingEntry,
    MissingOutput,
    BadTemplate,
    BadPath,
    BadDefine,
    BadVar,
    BadRules,
    BadLoader,
    BadLoaderPattern,
    BadGlob,
    LoadersUnordered,
    EmptyAxis,
    NoTasks,
    Unresolved,

    // ── plan validation ───────────────────────────────────────────────────────────
    OutputCollision,
    OutputInInputs,
    NoSuchTask,

    // ── execution ─────────────────────────────────────────────────────────────────
    MissingEntryFile,
    UndeclaredInput,
    DarkluaConfig,
    ProcessFailed,
    NoOutput,
    WriteFailed,
    Frozen,

    // ── lints ─────────────────────────────────────────────────────────────────────
    FoldBeforeInject,
    BranchBeforeFold,
    UnreachableDefine,
    UnrecordedReading,
    ShadowedVar,
    OutputOutsideRoot,
    StaleSchema,
    UnusedTemplate,
    IdenticalProfiles,
}

/// Every variant, so `explain` can list them and the CLI can resolve a slug.
pub const ALL: &[Code] = &[
    Code::NoManifest,
    Code::UnknownFormat,
    Code::Unreadable,
    Code::Syntax,
    Code::NotATable,
    Code::Eval,
    Code::Budget,
    Code::UnsetEnv,
    Code::UnknownBase,
    Code::CyclicExtends,
    Code::NameCollision,
    Code::BadName,
    Code::MissingEntry,
    Code::MissingOutput,
    Code::BadTemplate,
    Code::BadPath,
    Code::BadDefine,
    Code::BadVar,
    Code::BadRules,
    Code::BadLoader,
    Code::BadLoaderPattern,
    Code::BadGlob,
    Code::LoadersUnordered,
    Code::EmptyAxis,
    Code::NoTasks,
    Code::Unresolved,
    Code::OutputCollision,
    Code::OutputInInputs,
    Code::NoSuchTask,
    Code::MissingEntryFile,
    Code::UndeclaredInput,
    Code::DarkluaConfig,
    Code::ProcessFailed,
    Code::NoOutput,
    Code::WriteFailed,
    Code::Frozen,
    Code::FoldBeforeInject,
    Code::BranchBeforeFold,
    Code::UnreachableDefine,
    Code::UnrecordedReading,
    Code::ShadowedVar,
    Code::OutputOutsideRoot,
    Code::StaleSchema,
    Code::UnusedTemplate,
    Code::IdenticalProfiles,
];

impl Code {
    /// The stable kebab-case name, as printed in `error[missing-output]`.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::NoManifest => "no-manifest",
            Self::UnknownFormat => "unknown-format",
            Self::Unreadable => "unreadable",
            Self::Syntax => "syntax",
            Self::NotATable => "not-a-table",
            Self::Eval => "eval",
            Self::Budget => "budget",
            Self::UnsetEnv => "unset-env",
            Self::UnknownBase => "unknown-base",
            Self::CyclicExtends => "cyclic-extends",
            Self::NameCollision => "name-collision",
            Self::BadName => "bad-name",
            Self::MissingEntry => "missing-entry",
            Self::MissingOutput => "missing-output",
            Self::BadTemplate => "bad-template",
            Self::BadPath => "bad-path",
            Self::BadDefine => "bad-define",
            Self::BadVar => "bad-var",
            Self::BadRules => "bad-rules",
            Self::BadLoader => "bad-loader",
            Self::BadLoaderPattern => "bad-loader-pattern",
            Self::BadGlob => "bad-glob",
            Self::LoadersUnordered => "loaders-unordered",
            Self::EmptyAxis => "empty-axis",
            Self::NoTasks => "no-tasks",
            Self::Unresolved => "unresolved",
            Self::OutputCollision => "output-collision",
            Self::OutputInInputs => "output-in-inputs",
            Self::NoSuchTask => "no-such-task",
            Self::MissingEntryFile => "missing-entry-file",
            Self::UndeclaredInput => "undeclared-input",
            Self::DarkluaConfig => "darklua-config",
            Self::ProcessFailed => "process-failed",
            Self::NoOutput => "no-output",
            Self::WriteFailed => "write-failed",
            Self::Frozen => "frozen",
            Self::FoldBeforeInject => "fold-before-inject",
            Self::BranchBeforeFold => "branch-before-fold",
            Self::UnreachableDefine => "unreachable-define",
            Self::UnrecordedReading => "unrecorded-reading",
            Self::ShadowedVar => "shadowed-var",
            Self::OutputOutsideRoot => "output-outside-root",
            Self::StaleSchema => "stale-schema",
            Self::UnusedTemplate => "unused-template",
            Self::IdenticalProfiles => "identical-profiles",
        }
    }

    /// Fixed per code, so a finding cannot be reported at the wrong level by accident.
    pub const fn severity(self) -> Severity {
        match self {
            Self::UnreachableDefine
            | Self::UnrecordedReading
            | Self::ShadowedVar
            | Self::OutputOutsideRoot
            | Self::StaleSchema
            | Self::UnusedTemplate
            | Self::IdenticalProfiles => Severity::Warning,
            _ => Severity::Error,
        }
    }

    /// What a run exits with when this is the worst thing that happened.
    ///
    /// Distinct from [`Self::recoverable`], which is about whether a *phase* can keep
    /// collecting. Reporting every bad profile in one run and exiting 2 because the
    /// manifest did not resolve are two different questions.
    pub const fn exit(self) -> Exit {
        match self {
            Self::MissingEntryFile
            | Self::UndeclaredInput
            | Self::DarkluaConfig
            | Self::ProcessFailed
            | Self::NoOutput
            | Self::WriteFailed
            | Self::Frozen => Exit::Build,

            Self::FoldBeforeInject
            | Self::BranchBeforeFold
            | Self::UnreachableDefine
            | Self::UnrecordedReading
            | Self::ShadowedVar
            | Self::OutputOutsideRoot
            | Self::StaleSchema
            | Self::UnusedTemplate
            | Self::IdenticalProfiles => Exit::Lint,

            // Everything else is the manifest failing to load or to resolve, which
            // includes a selection naming a task the manifest does not describe.
            _ => Exit::Config,
        }
    }

    pub fn parse(slug: &str) -> Option<Self> {
        ALL.iter().copied().find(|code| code.slug() == slug)
    }

    /// The long form, printed by `pcmp explain <CODE>`.
    ///
    /// One exhaustive match rather than several: the compiler checking that every code
    /// has a description is the whole reason this is not a lookup table.
    #[allow(
        clippy::too_many_lines,
        reason = "a catalogue is as long as the catalogue"
    )]
    pub const fn description(self) -> &'static str {
        match self {
            Self::NoManifest => {
                "\
No manifest was found in the working directory or any directory above it. `pcmp` looks
for pcmp.json5, pcmp.json, pcmp.jsonc, pcmp.toml and pcmp.luau, in that order, at each
level. Run `pcmp init` to write one, or point at an existing manifest with `-m`."
            }

            Self::UnknownFormat => {
                "\
The manifest's extension does not name a format `pcmp` can read. Format comes from the
extension and never from the content, so a JSON manifest called `pcmp.conf` is not
readable. Supported: json5, json, jsonc, toml, luau."
            }

            Self::Unreadable => {
                "\
A file could not be read or a directory could not be listed. The underlying operating
system error is attached to the diagnostic."
            }

            Self::Syntax => {
                "\
The manifest is not valid in the format its extension declares. The parser's own message
follows, with a line and column where the parser reports one."
            }

            Self::NotATable => {
                "\
A Luau manifest must evaluate to a table. It returned something else — commonly a
missing `return`, which makes the chunk evaluate to nil."
            }

            Self::Eval => {
                "\
A Luau manifest raised an error while evaluating. The Luau traceback follows."
            }

            Self::Budget => {
                "\
A Luau manifest exceeded its evaluation budget or its memory limit. A manifest describes
a build; it is not the place for unbounded work. The usual cause is a loop whose
condition never becomes false."
            }

            Self::UnsetEnv => {
                "\
`pcmp.env(name)` was called for a variable that is set neither by `--env` nor in the
process environment. Pass `--env NAME=VALUE`, or use `pcmp.envOr(name, fallback)` when a
default is acceptable."
            }

            Self::UnknownBase => {
                "\
`extends` names a profile or template that does not exist. Both maps share one namespace,
so the name is looked up in `templates` and in `profiles`."
            }

            Self::CyclicExtends => {
                "\
An `extends` chain returns to a profile it already passed through, so it has no base to
resolve from. The cycle is listed in the help line."
            }

            Self::NameCollision => {
                "\
The same name appears in both `templates` and `profiles`. The two maps share one
namespace so that `extends` needs no precedence rule, which means a name may appear in
only one of them."
            }

            Self::BadName => {
                "\
A profile or template name contains `[`, `]`, `,` or `=`. Those characters delimit a
task identifier such as `dist[target=roblox]`, so a name containing one could not be
selected on the command line unambiguously."
            }

            Self::MissingEntry => {
                "\
The profile declares no `entry`, and neither does anything it extends. `entry` is a file
to bundle or a directory to process as a tree."
            }

            Self::MissingOutput => {
                "\
The profile declares no `output`, and neither does anything it extends. `output` is a
template, so it may vary by profile or axis: \"dist/{profile}/app.luau\"."
            }

            Self::BadTemplate => {
                "\
An `entry`, `output` or `header` template refers to a token that is not a var, not an
axis and not `{profile}`, or leaves a `{` unclosed, or expands to nothing. Write `{{` and
`}}` for literal braces. An unknown token is an error rather than an empty string,
because a silently empty path segment is far harder to notice."
            }

            Self::BadPath => {
                "\
A path is empty, or climbs above the filesystem root with `..`. Paths resolve against
the manifest's own directory, never the working directory."
            }

            Self::BadDefine => {
                "\
A define key is not a Luau identifier, or its value cannot be represented. darklua's
`inject_global_value` substitutes by name, so a key such as `my-flag` or `end` would
match nothing. Values must be finite, and an integer must survive a round trip through
an IEEE double, which bounds it at 2^53."
            }

            Self::BadVar => {
                "\
A var name is not a Luau identifier, or two var names collide as one constant. Every var
becomes both a `{token}` and a `PCMP_<NAME>` global, and the constant is uppercased, so
`channel` and `Channel` would produce the same global."
            }

            Self::BadRules => {
                "\
`darklua.rules` is not a list darklua could read. Each entry is a rule name, or an object
with a `rule` key and that rule's own settings."
            }

            Self::BadLoader => {
                "\
A loader names a strategy darklua does not have. Valid: copy, skip, luau, json,
json_lines, toml, yaml, string, buffer, bytes — and the encoded forms string/base64,
string/zstd, string/gzip and string/zlib, with buffer and bytes likewise."
            }

            Self::BadLoaderPattern => {
                "\
A loader's `pattern` is not a valid filter pattern. Patterns match a file's path relative
to the entry."
            }

            Self::BadGlob => {
                "\
An `ignore` entry is not a valid glob. Globs match each file's path relative to the root
it was found under."
            }

            Self::LoadersUnordered => {
                "\
A Luau manifest declared `loaders` as a table keyed by pattern. darklua takes the first
matching pattern, and a Luau table has no order, so which loader wins would be decided by
hash order. Write an array of `{ pattern = ..., use = ... }` instead. Data formats may
use either spelling, because their maps preserve the order they were written in."
            }

            Self::EmptyAxis => {
                "\
An axis lists no values, so its profile expands to zero tasks. Remove the axis, or give
it values."
            }

            Self::NoTasks => {
                "\
The manifest declares no profiles, so there is nothing to build. A `templates` entry is
never built on its own."
            }

            Self::Unresolved => {
                "\
The manifest was read, but the plan it describes could not be built. The findings printed
above this one say why; this is only the summary. Nothing was built."
            }

            Self::OutputCollision => {
                "\
Two tasks write to the same path. They would race, and whichever finished last would
win. Give them distinct `output` templates — `{profile}` and every axis are available as
tokens."
            }

            Self::OutputInInputs => {
                "\
One task writes inside another task's source roots, so a build would feed its own output
back in as an input. Move the output outside every root, or exclude it with `ignore`."
            }

            Self::NoSuchTask => {
                "\
No task matched the selection. A selector is a profile name or an exact task identifier;
use `--axis KEY=VALUE` to filter an expansion by coordinate."
            }

            Self::MissingEntryFile => {
                "\
The task's `entry` does not exist on disk. The path resolves against the manifest's
directory, not the working directory."
            }

            Self::UndeclaredInput => {
                "\
darklua asked for a file that is not in this task's staged input set. A build reads only
what the manifest declares, so a file outside every root cannot be reached — which is
what stops an undeclared dependency from silently deciding the output. Add the directory
holding it to `sources`."
            }

            Self::DarkluaConfig => {
                "\
darklua rejected the configuration this task compiles to. The emitted configuration is
printed with the error; `pcmp plan <TASK>` shows the same thing without building."
            }

            Self::ProcessFailed => {
                "\
darklua reported an error while transforming this task's sources."
            }

            Self::NoOutput => {
                "\
The task reported no failure but produced nothing. A file filter matching nothing is the
usual cause: `apply_to_files` and `skip_files` match each file's path relative to the
entry, so `src/**` matches nothing when the entry is already `src/init.luau`."
            }

            Self::WriteFailed => {
                "\
An artifact could not be committed to disk. Artifacts are written to a temporary file and
renamed into place, so a failure here leaves the previous artifact intact."
            }

            Self::Frozen => {
                "\
A `--frozen` build did not reproduce what `pcmp.lock` records. Either the plan resolved
differently from the one the lock describes — the manifest changed since it was written —
or a task produced different bytes from the same inputs. The differing tasks are named."
            }

            Self::FoldBeforeInject => {
                "\
`compute_expression` is scheduled before `inject_global_value`. Folding cannot see a
value substituted after it runs, so the define has no effect. Every injection must
precede every fold; `pcmp` places the injections it generates first, so this only arises
when a manifest writes its own."
            }

            Self::BranchBeforeFold => {
                "\
`remove_unused_if_branch` is scheduled before `compute_expression`. A branch can only be
removed once its condition has folded to a constant, so the branch survives."
            }

            Self::UnreachableDefine => {
                "\
A define's identifier appears in none of the task's sources, so nothing will be
substituted. Almost always a typo in the define's name or in the source that meant to
read it."
            }

            Self::UnrecordedReading => {
                "\
The manifest read the clock or the environment, and no pcmp.lock exists. The build is
reproducible only relative to those readings, and nothing records what they were. Run
`pcmp build --lock` to write them down; `pcmp build --frozen` then reproduces the build
exactly, timestamps included."
            }

            Self::ShadowedVar => {
                "\
A var is named `profile`, which `pcmp` also sets. The built-in wins, so the declared
value is never used. Rename it."
            }

            Self::OutputOutsideRoot => {
                "\
A task writes outside the manifest's directory. Legal, and occasionally intended, but it
means `pcmp` is modifying files no one reading the manifest would expect it to touch."
            }

            Self::StaleSchema => {
                "\
A pcmp.schema.json in the project differs from the schema this binary generates, so
editor completion is describing a different version of the manifest format. Regenerate it
with `pcmp schema`, or delete it — it is not required."
            }

            Self::UnusedTemplate => {
                "\
A template is never extended and is never a base. Templates are not built, so this one
does nothing. Remove it, or move it to `profiles` so it builds."
            }

            Self::IdenticalProfiles => {
                "\
Two profiles resolve to the same task apart from their output. Extract what they share
into a template and `extends` it, or give one of them an axis."
            }
        }
    }
}

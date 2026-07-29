---
title: Diagnostics
description: Every code pcmp can report, and what to do about it.
icon: lucide/circle-alert
---

# Diagnostics

A code names one failure and never another. A message may be reworded, a code may not be reused.

```sh
pcmp explain missing-output
```

!!! info "Generated from the binary"

    `pcmp explain --format markdown` writes this page, and CI fails when the committed copy stops matching what the binary would print. Nothing here can describe a version of `pcmp` you are not running.

## Reading the command line

Reported before anything is read. `clap` rejects an unknown flag or an unknown value on its own, so what is left here is an argument whose *shape* is wrong.

### bad-argument

`error`
:   Exits `2`.

An argument's value is not the shape its flag takes. `--env`, `--var`, `--define` and
`--axis` each take `KEY=VALUE`, `--now` takes an RFC 3339 instant in UTC to the second,
and `pcmp explain` takes a code from `pcmp explain` with no argument.

## Reading the manifest

Nothing can be collected past a manifest that will not parse, so any of these stops the run on its own.

### no-manifest

`error`
:   Exits `2`.

No manifest was found in the working directory or any directory above it. `pcmp` looks
for pcmp.json5, pcmp.json, pcmp.jsonc, pcmp.toml and pcmp.luau, in that order, at each
level. Run `pcmp init` to write one, or point at an existing manifest with `-m`.

### unknown-format

`error`
:   Exits `2`.

The manifest's extension does not name a format `pcmp` can read. Format comes from the
extension and never from the content, so a JSON manifest called `pcmp.conf` is not
readable. Supported: json5, json, jsonc, toml, luau.

### unreadable

`error`
:   Exits `2`.

The operating system refused a read. Its own message follows.

### syntax

`error`
:   Exits `2`.

The manifest is not valid in the format its extension declares. The parser's own message
follows, with a line and column where it reports one.

### not-a-table

`error`
:   Exits `2`.

A Luau manifest must evaluate to a table. It returned something else, commonly a
missing `return`, which makes the chunk evaluate to nil.

### eval

`error`
:   Exits `2`.

A Luau manifest raised an error while evaluating. Its traceback follows.

### budget

`error`
:   Exits `2`.

A Luau manifest exceeded its evaluation budget or its memory limit. A manifest describes
a build. It is not the place for unbounded work. The usual cause is a loop whose
condition never becomes false.

### unset-env

`error`
:   Exits `2`.

`pcmp.env(name)` was called for a variable that is set neither by `--env` nor in the
process environment. Pass `--env NAME=VALUE`, or use `pcmp.envOr(name, fallback)` when a
default is acceptable.

## Resolving the plan

Every profile is checked before the run gives up, so one edit can fix a page of these.

### unknown-base

`error`
:   Exits `2`.

`extends` names a profile or template that does not exist. Both maps share one namespace,
so the name is looked up in `templates` and in `profiles`.

### cyclic-extends

`error`
:   Exits `2`.

An `extends` chain returns to a profile it already passed through, so it has no base to
resolve from. The cycle is listed in the help line.

### name-collision

`error`
:   Exits `2`.

The same name appears in both `templates` and `profiles`. The two maps share one
namespace so that `extends` needs no precedence rule, which means a name may appear in
only one of them.

### bad-name

`error`
:   Exits `2`.

A profile or template name contains `[`, `]`, `,` or `=`. Those characters delimit a
task identifier such as `dist[target=roblox]`, so a name containing one could not be
selected on the command line unambiguously.

### missing-entry

`error`
:   Exits `2`.

Neither the profile nor anything it extends declares an `entry`, which is a file to
bundle or a directory to process as a tree.

### missing-output

`error`
:   Exits `2`.

Neither the profile nor anything it extends declares an `output`, which is a template and
so may vary by profile or axis: "dist/{profile}/app.luau".

### bad-template

`error`
:   Exits `2`.

An `entry`, `output` or `header` template refers to a token that is not a var, not an
axis and not `{profile}`, or leaves a `{` unclosed, or expands to nothing. Write `{{` and
`}}` for literal braces. An unknown token is an error rather than an empty string,
because a silently empty path segment is far harder to notice.

In `entry`, `output` and `sources` a token also may not expand to a `.` or `..` segment.
`dist/{profile}/app.luau` under a profile named `..` would write `app.luau`, outside the
directory the template names. A plain `/` is allowed, so `{outdir}/app.luau` still works.

### bad-path

`error`
:   Exits `2`.

A path is empty, or climbs above the filesystem root with `..`. Paths resolve against
the manifest's own directory, never the working directory.

### bad-define

`error`
:   Exits `2`.

A define key is not a Luau identifier, or its value cannot be represented. darklua's
`inject_global_value` substitutes by name, so a key such as `my-flag` or `end` would
match nothing. Values must be finite, and an integer must survive a round trip through
an IEEE double, which bounds it at 2^53.

### bad-var

`error`
:   Exits `2`.

A var name is not a Luau identifier, or two var names collide as one constant. Every var
becomes both a `{token}` and a `PCMP_<NAME>` global, and the constant is uppercased, so
`channel` and `Channel` would produce the same global.

### bad-rules

`error`
:   Exits `2`.

`darklua.rules` is not a list darklua could read. Each entry is a rule name, or an object
with a `rule` key and that rule's own settings.

### bad-loader

`error`
:   Exits `2`.

A loader names a strategy darklua does not have. Valid: copy, skip, luau, json,
json_lines, toml, yaml, string, buffer, bytes, and the encoded forms string/base64,
string/zstd, string/gzip and string/zlib, with buffer and bytes likewise.

### bad-loader-pattern

`error`
:   Exits `2`.

A loader's `pattern` is not one darklua accepts. A pattern matches a file's path relative
to the entry.

### bad-glob

`error`
:   Exits `2`.

An `ignore` entry is not a valid glob. Globs match each file's path relative to the root
it was found under.

### empty-axis

`error`
:   Exits `2`.

An axis lists no values, so its profile expands to zero tasks.

### no-tasks

`error`
:   Exits `2`.

The manifest declares no profiles, so there is nothing to build. A `templates` entry is
never built on its own.

## Checking the plan

Whole-plan problems, found once every task is known.

### output-collision

`error`
:   Exits `2`.

Two tasks write to the same path. They would race, and whichever finished last would
win. Give them distinct `output` templates, `{profile}` and every axis are available as
tokens.

### output-in-inputs

`error`
:   Exits `2`.

One task writes inside another task's source roots, so a build would feed its own output
back in as an input. Move the output outside every root, or exclude it with `ignore`.

### no-such-task

`error`
:   Exits `2`.

No task matched the selection. A selector is a profile name or an exact task identifier,
and `--axis KEY=VALUE` filters an expansion by coordinate. There is no wildcard, because
a task identifier contains `[`, `]`, `=` and `,`, which every glob dialect reads as
syntax.

## Building

Reported per task. One failing task does not stop the others.

### missing-entry-file

`error`
:   Exits `1`.

The task's `entry` does not exist on disk. The path resolves against the manifest's
directory, not the working directory.

### undeclared-input

`error`
:   Exits `1`.

darklua asked for a file that is not in this task's staged input set. A build reads only
what the manifest declares, so a file outside every root cannot be reached, which is
what stops an undeclared dependency from silently deciding the output. Add the directory
holding it to `sources`.

### darklua-config

`error`
:   Exits `1`.

darklua rejected the configuration this task compiles to. The emitted configuration is
printed with the error, and `pcmp plan <TASK>` shows the same thing without building.

### process-failed

`error`
:   Exits `1`.

darklua reported an error while transforming this task's sources. Its own message
follows. When the error is a file darklua could not find, the code is
`undeclared-input` instead.

### no-output

`error`
:   Exits `1`.

The task reported no failure but produced nothing. A file filter matching nothing is the
usual cause: `apply_to_files` and `skip_files` match each file's path relative to the
entry, so `src/**` matches nothing when the entry is already `src/init.luau`.

### write-failed

`error`
:   Exits `1`.

An artifact could not be committed to disk. Artifacts are written to a temporary file and
renamed into place, so a failure here leaves the previous artifact intact.

### frozen

`error`
:   Exits `1`.

A `--frozen` build did not reproduce what `pcmp.lock` records. Either the plan resolved
differently from the one the lock describes, the manifest changed since it was written,
or a task produced different bytes from the same inputs. The differing tasks are named.

## Lints

Reported by `pcmp check`. Most are warnings, which `--strict` makes fail as well. The two rule-order codes are errors, because a rule that cannot fire is not a matter of taste.

### fold-before-inject

`error`
:   Exits `3`.

`compute_expression` is scheduled before `inject_global_value`. Folding cannot see a
value substituted after it runs, so the define has no effect. Every injection must
precede every fold. `pcmp` places the injections it generates first, so this only arises
when a manifest writes its own.

### branch-before-fold

`error`
:   Exits `3`.

`remove_unused_if_branch` is scheduled before `compute_expression`. A branch can only be
removed once its condition has folded to a constant, so the branch survives.

### unreachable-define

`warning`
:   Exits `3`.

A define's identifier appears in none of the task's sources, so nothing will be
substituted. Almost always a typo in the define's name or in the source that meant to
read it.

### unrecorded-reading

`warning`
:   Exits `3`.

The manifest read the clock or the environment, and no pcmp.lock exists. The build is
reproducible only relative to those readings, and nothing records what they were. Run
`pcmp build --lock` to write them down, and `pcmp build --frozen` then reproduces the build
exactly, timestamps included.

### shadowed-var

`warning`
:   Exits `3`.

A var is named `profile`, which `pcmp` also sets. The built-in wins, so the declared
value is never used. Rename it.

### output-outside-root

`warning`
:   Exits `3`.

A task writes outside the manifest's directory. Legal, and occasionally intended, but it
means `pcmp` is modifying files no one reading the manifest would expect it to touch.

### stale-schema

`warning`
:   Exits `3`.

A pcmp.schema.json in the project differs from the schema this binary generates, so
editor completion is describing a different version of the manifest format. Regenerate it
with `pcmp schema`, or delete it, it is not required.

### unused-template

`warning`
:   Exits `3`.

A template is never extended and is never a base. Templates are not built, so this one
does nothing. Remove it, or move it to `profiles` so it builds.

### identical-profiles

`warning`
:   Exits `3`.

Two profiles resolve to the same task apart from their output. Extract what they share
into a template and `extends` it, or give one of them an axis.


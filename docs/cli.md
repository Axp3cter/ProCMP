---
description: Every command, flag and exit code.
---

# CLI

```
pcmp [OPTIONS] <COMMAND>
```

<table><thead><tr><th width="260">Global flag</th><th></th></tr></thead><tbody>
<tr><td><code>-m</code>, <code>--manifest &#60;PATH&#62;</code></td><td>Manifest to use. Discovered otherwise</td></tr>
<tr><td><code>--cache-dir &#60;PATH&#62;</code></td><td>Where build state is kept. Defaults to <code>.pcmp/</code></td></tr>
<tr><td><code>-e</code>, <code>--env &#60;KEY=VALUE&#62;</code></td><td>A value for <code>pcmp.env</code>, ahead of the process environment</td></tr>
<tr><td><code>--var &#60;KEY=VALUE&#62;</code></td><td>Set a token. Beats the manifest</td></tr>
<tr><td><code>-D</code>, <code>--define &#60;KEY=VALUE&#62;</code></td><td>Set a constant. Beats the manifest</td></tr>
<tr><td><code>--json</code></td><td>Machine-readable output. Works on every command</td></tr>
</tbody></table>

All are repeatable where a value is taken. `--env` is read by the manifest but never
exported, so it cannot reach anything ProCMP runs.

## `plan`

Resolve and print without building.

```
$ pcmp plan
2 task(s), plan a18350afadee

  debug    dist/debug/app.luau     7 rules
  release  dist/release/app.luau  11 rules
```

Touches no source files, so this is exactly what a build would do. The digest covers
every task, so two checkouts can be compared without building either.

## `build [TASKS]`

```sh
pcmp build                         # everything
pcmp build release                 # one task or profile
pcmp build 'dist[target=roblox]'   # one matrix task, by exact identifier
pcmp build '*target=roblox*'       # across a matrix axis
pcmp build --pick                  # choose from a menu
pcmp build --no-cache              # rebuild regardless of cached state
```

`*` is the only wildcard. A matrix identifier holds `[`, `]` and `=`, which every glob
dialect would read as syntax.

Tasks run in parallel. Failures are collected rather than aborting, so one broken
profile does not hide the others. A selection matching nothing is an error, not a no-op.

### `--pick`

A menu on stderr. Arrows move, enter chooses. Every action is a row, so there is no
legend to read.

```
  build

    [x] debug                          dist/debug/app.luau
    [ ] release                        dist/release/app.luau
    [x] dist[flavour=min,target=lune]  dist/lune/min.luau

  > Select all
    Select none
    Continue with 2 selected
    Cancel
```

Runs only when the flag is given and only when stdin and stderr are both terminals, so
a script or a CI job is never asked a question it cannot answer.

## `check`

```sh
pcmp check
pcmp check --strict   # also fail on warnings
```

See [Diagnostics](diagnostics.md).

## `verify`

Builds twice with the cache off, then byte-compares every artifact. A directory output
compares the whole tree.

```
$ pcmp verify
reproducible: 2 task(s) byte-identical across two builds
```

Names the differing tasks if anything changed. A build that fails is reported as a build
failure, not as a comparison that could not be made.

## `watch [TASKS]`

Builds, then rebuilds whenever an input or the manifest changes. Accepts the same
selectors as `build`, including `--pick`.

The watched set is the same one the cache is keyed on, so nothing can wake the watcher
without invalidating a build, or the reverse. The manifest is re-read each cycle, so
editing it takes effect without a restart, including an edit that breaks it.

## `explain [TASK]`

Entry, output, digest, every var, every define with its type, and the darklua
configuration the task compiles to. `--pick` chooses from a menu instead of naming one.

## `init`

Writes a manifest and its schema. Nothing else: no directories, no prompts, no
`.gitignore` rewriting.

```
$ pcmp init
created  pcmp.json5
created  pcmp.schema.json

next     pcmp plan
```

`--format luau` writes `pcmp.luau` and `pcmp.d.luau` instead. The entry point is
detected from `src/init.luau`, `src/main.luau`, `init.luau` or `main.luau`. It refuses
to overwrite, so it is safe to run twice.

## `schema`

```sh
pcmp schema                 # JSON Schema
pcmp schema --format luau   # Luau type definitions
```

Needs no project. See [Install](install.md#editor-completion).

## Inputs and caching

A task is skipped when its configuration, every input, and the linked darklua are all
unchanged. What counts as an input is derived from the plan, not guessed at from
directory names.

**Hashed:** every file under the manifest's directory and under every `sources` root,
whatever its extension. There is no allowlist, because a content loader can make a
`.json` or a `.png` a real input.

**Not hashed:** every task's `output`, the cache directory, `.git`, and anything
matching `ignore`. Nothing else is assumed, so `dist` and `node_modules` are not special
names.

```json5
sources: ["../shared"],            // extra roots
ignore: ["**/Packages/**"],        // wax globs, relative to each root
```

`ignore` is a speed control, not a correctness one. Ignoring a directory the build reads
means edits there will not rebuild.

Editing one file rebuilds every task. darklua follows requires, so a per-task input list
would have to guess at what a module pulls in.

## Exit codes

<table><thead><tr><th width="90">Code</th><th></th></tr></thead><tbody>
<tr><td><code>0</code></td><td>Success</td></tr>
<tr><td><code>1</code></td><td>A build task failed, or output was not reproducible</td></tr>
<tr><td><code>2</code></td><td>The manifest could not be loaded or resolved</td></tr>
<tr><td><code>5</code></td><td>Linting failed</td></tr>
</tbody></table>

## In CI

{% code title=".github/workflows/build.yml" %}
```yaml
- run: pcmp check --strict
- run: pcmp build  --var version=${{ github.ref_name }}
- run: pcmp verify --var version=${{ github.ref_name }}
```
{% endcode %}

```sh
pcmp check --json  | jq '[.[] | select(.severity == "deny")] | length'
pcmp build --json  | jq '.tasks[] | select(.outcome.status == "failed")'
pcmp verify --json | jq '.reproducible'
```

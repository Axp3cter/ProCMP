---
description: Every command, flag and exit code.
---

# CLI

```
pcmp [OPTIONS] <COMMAND>
```

<table><thead><tr><th width="270">Global flag</th><th></th></tr></thead><tbody>
<tr><td><code>-m</code>, <code>--manifest &#60;PATH&#62;</code></td><td>Manifest to use. Discovered otherwise.</td></tr>
<tr><td><code>--cache-dir &#60;PATH&#62;</code></td><td>Where build state is kept. Defaults to <code>.pcmp/</code> beside the manifest.</td></tr>
<tr><td><code>--var &#60;KEY=VALUE&#62;</code></td><td>Set a token. Repeatable. Beats the manifest.</td></tr>
<tr><td><code>-D</code>, <code>--define &#60;KEY=VALUE&#62;</code></td><td>Set a constant. Repeatable. Beats the manifest.</td></tr>
<tr><td><code>-e</code>, <code>--env &#60;KEY=VALUE&#62;</code></td><td>A value for <code>pcmp.env</code>, ahead of the process environment. Repeatable.</td></tr>
<tr><td><code>--json</code></td><td>Machine-readable output. Works on every command.</td></tr>
</tbody></table>

Discovery looks for `pcmp.luau`, `pcmp.json`, `pcmp.jsonc`, `pcmp.json5` then
`pcmp.toml`, in the working directory and then each of its ancestors, so `pcmp` works
from anywhere inside a project.

`--var` and `--define` apply after inheritance and work whatever the manifest format.
`--env` is read by the manifest but never exported, so a value passed there cannot reach
anything ProCMP runs.

```sh
pcmp build --var version="$(git describe --tags)" -D CHANNEL=beta
```

## `plan`

Resolve the manifest and print the plan without building.

```
$ pcmp plan
2 task(s), plan a18350afadee

  debug    dist/debug/app.luau     7 rules
  release  dist/release/app.luau  11 rules
```

Resolution touches no source files, so this is exactly what a build would do. The plan
digest covers every task, so two checkouts can be compared without building either.

## `build`

```sh
pcmp build                         # everything
pcmp build release                 # one task or profile
pcmp build debug release           # several
pcmp build 'dist[target=roblox]'   # one matrix task, by exact identifier
pcmp build '*target=roblox*'       # across a matrix axis
pcmp build --no-cache              # rebuild regardless of cached state
```

`*` stands for any run of characters and is the only wildcard. A matrix identifier
contains `[`, `]` and `=`, which every glob dialect would read as syntax.

Tasks run in parallel. Failures are collected rather than aborting, so one broken profile
does not hide the others. A filter matching nothing is an error, not a no-op.

## `check`

```sh
pcmp check
pcmp check --strict   # also fail on warnings
```

Exits `5` on any error. See [Diagnostics](diagnostics.md).

## `verify`

Build twice with the cache off, then byte-compare every artifact.

```
$ pcmp verify
reproducible: 2 task(s) byte-identical across two builds
```

Exits `1` and names the differing tasks if anything changed between runs. A build that
fails is reported as a build failure, not as a comparison that could not be made.

## `watch [TASKS]`

Builds, then rebuilds whenever an input or the manifest changes. Accepts the same
patterns as `build`.

```
$ pcmp watch release
  built   release  dist/release/app.luau  (2 ms)
1 built, 0 cached, 0 failed
  built   release  dist/release/app.luau  (1 ms)
1 built, 0 cached, 0 failed
```

The watched set is the same one the cache is keyed on, so nothing can wake the watcher
without invalidating a build, or the reverse. See [Inputs](inputs.md).

The manifest is re-read each cycle, so editing it takes effect without a restart. That
includes an edit that breaks it, which is reported and then waited on.

## `init`

Writes `pcmp.luau` and `pcmp.d.luau`. Nothing else: no directories, no prompts, no
`.gitignore` rewriting.

```
$ pcmp init
created  pcmp.luau
created  pcmp.d.luau

next     pcmp plan
```

The entry point is detected from `src/init.luau`, `src/main.luau`, `init.luau` or
`main.luau`. Pass `--entry` otherwise. It refuses to overwrite an existing manifest, so
it is safe to run twice.

## `explain <TASK>`

Entry, output, digest, every var, every define with its type, and the darklua
configuration the task compiles to. `--json` emits the task and that configuration
together.

## `schema`

```sh
pcmp schema                 # JSON Schema
pcmp schema --format luau   # Luau type definitions
```

Needs no project, so run it anywhere. See [Install](install.md#editor-completion).

## Exit codes

<table><thead><tr><th width="90">Code</th><th></th></tr></thead><tbody>
<tr><td><code>0</code></td><td>Success.</td></tr>
<tr><td><code>1</code></td><td>A build task failed, or output was not reproducible.</td></tr>
<tr><td><code>2</code></td><td>The manifest could not be loaded or resolved.</td></tr>
<tr><td><code>5</code></td><td>Linting failed.</td></tr>
</tbody></table>

## Caching

Build state lives in `.pcmp/` beside the manifest. Add it to `.gitignore`, or move it
with `--cache-dir`.

A task is skipped when its configuration, every input, and the darklua version are all
unchanged. What counts as an input is derived from the plan rather than guessed at from
directory names.

{% content-ref url="inputs.md" %}
[inputs.md](inputs.md)
{% endcontent-ref %}

## In CI

{% code title=".github/workflows/build.yml" %}
```yaml
- run: pcmp check --strict
- run: pcmp build  --var version=${{ github.ref_name }}
- run: pcmp verify --var version=${{ github.ref_name }}
```
{% endcode %}

`--json` makes results parseable:

```sh
pcmp check --json  | jq '[.[] | select(.severity == "deny")] | length'
pcmp build --json  | jq '.tasks[] | select(.outcome.status == "failed")'
pcmp verify --json | jq '.reproducible'
```

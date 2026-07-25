---
description: Every command, flag and exit code.
---

# CLI

```
pcmp [OPTIONS] <COMMAND>
```

## Global flags

Available on every command, and all repeatable.

| Flag | |
| --- | --- |
| `-m`, `--manifest <PATH>` | Manifest to use. Discovered otherwise |
| `--cache-dir <PATH>` | Where build state is kept. Defaults to `.pcmp/` |
| `-e`, `--env <KEY=VALUE>` | A value for `pcmp.env`, ahead of the process environment |
| `--var <KEY=VALUE>` | Set a token. Beats the manifest |
| `-D`, `--define <KEY=VALUE>` | Set a constant. Beats the manifest |
| `--json` | Machine-readable output |

{% hint style="info" %}
`--env` is passed to the manifest explicitly, never exported, so a value given here
cannot reach anything ProCMP spawns.
{% endhint %}

## `plan`

Resolve and print without building. Touches no source file.

```
$ pcmp plan --var version=v1.0.0
2 task(s), plan e070fcff9252

  dev      dist/dev/app.luau      5 rules
  release  dist/release/app.luau  9 rules
```

The digest covers every task, so it changes when anything about the plan does.

## `build [TASKS]`

```sh
pcmp build                         # everything
pcmp build release                 # one task or profile
pcmp build 'dist[target=roblox]'   # one matrix task, by exact identifier
pcmp build '*target=roblox*'       # across a matrix axis
pcmp build --pick                  # choose from a menu
pcmp build --no-cache              # rebuild regardless of cached state
```

```
$ pcmp build --var version=v1.0.0
  built   dev      dist/dev/app.luau      (12 ms)
  built   release  dist/release/app.luau  (14 ms)

2 built, 0 cached, 0 failed
```

Tasks run in parallel. A failure is collected rather than aborted on, and a selection
matching nothing is an error.

{% hint style="info" %}
`*` is the only wildcard, because a matrix identifier holds `[`, `]` and `=`, which
every glob dialect would read as syntax.
{% endhint %}

### `--pick`

A menu on stderr. Arrows move, enter chooses.

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

Accepted by `build`, `watch`, `verify` and `explain`. Requires stdin and stderr to both
be terminals, and naming tasks as well as passing `--pick` is rejected.

## `check`

```sh
pcmp check
pcmp check --strict   # also fail on warnings
```

```
$ pcmp check
no findings
```

See [Diagnostics](diagnostics.md).

## `verify [TASKS]`

Builds twice with the cache off, then byte-compares every artifact. A directory output
compares the whole tree. Accepts the same selectors as `build`.

```
$ pcmp verify
reproducible: 2 task(s) byte-identical across two builds
```

Names the differing tasks if anything changed.

## `watch [TASKS]`

Builds, then rebuilds whenever an input or the manifest changes. Accepts the same
selectors as `build`.

The watched set is the one the cache is keyed on. The manifest is re-read each cycle,
including an edit that breaks it.

## `explain [TASK]`

Entry, output, digest, every var, every define with its type, and the darklua
configuration the task compiles to.

```
$ pcmp explain release --var version=v1.0.0
task     release
entry    src/init.luau
output   dist/release/app.luau
digest   bb38e35e4db8

vars
  name     app
  profile  release
  version  v1.0.0

defines
  DEBUG         bool:false
  PCMP_NAME     string:app
  PCMP_PROFILE  string:release
  PCMP_VERSION  string:v1.0.0
```

## `init`

Writes a manifest and its schema. Nothing else: no directories, no prompts, no
`.gitignore` rewriting.

```
$ pcmp init
created  pcmp.json5
created  pcmp.schema.json

next     pcmp plan
```

`--format luau` writes `pcmp.luau` and `pcmp.d.luau` instead. The entry point is detected
from `src/init.luau`, `src/main.luau`, `init.luau` or `main.luau`, or given with
`--entry`.

{% hint style="info" %}
Only the current directory is checked, so a project above does not block a nested one.
Refuses to overwrite either file.
{% endhint %}

## `schema`

```sh
pcmp schema                 # JSON Schema
pcmp schema --format luau   # Luau type definitions
```

Needs no project. See [Editor completion](install.md#editor-completion).

## Inputs and caching

A task is skipped when its configuration, every input, and the linked darklua are all
unchanged.

| | |
| --- | --- |
| **Hashed** | Every file under the manifest's directory and under every `sources` root, whatever its extension |
| **Not hashed** | Every task's `output`, the cache directory, `.git`, and anything matching `ignore` |

```json5
sources: ["../shared"],            // extra roots
ignore: ["**/Packages/**"],        // wax globs, relative to each root
```

{% hint style="warning" %}
Extension is never a filter, because a content loader can make a `.json` or a `.png` a
real build input. By the same token, ignoring a directory the build reads means edits
there will not rebuild.
{% endhint %}

## Exit codes

| Code | |
| --- | --- |
| `0` | Success |
| `1` | A build task failed, or output was not reproducible |
| `2` | The manifest could not be loaded or resolved |
| `5` | Linting failed |

## In CI

{% code title=".github/workflows/build.yml" %}
```yaml
- run: pcmp check --strict
- run: pcmp build  --var version=${{ github.ref_name }}
- run: pcmp verify --var version=${{ github.ref_name }}
```
{% endcode %}

`--json` works on every command:

```sh
pcmp check --json  | jq '[.[] | select(.severity == "deny")] | length'
pcmp build --json  | jq '.tasks[] | select(.outcome.status == "failed")'
pcmp verify --json | jq '.reproducible'
```

{% content-ref url="diagnostics.md" %}
[diagnostics.md](diagnostics.md)
{% endcontent-ref %}

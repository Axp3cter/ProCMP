---
description: Selecting tasks, and what the exit codes mean.
---

# CLI

`pcmp help` lists every command and flag, and `pcmp explain <CODE>` describes any
diagnostic. This page covers only what those cannot: how selection works, and what a
process exit means.

## Selecting tasks

`build` and `watch` take a selection. Without one they take everything.

```sh
pcmp build release                             # a profile
pcmp build 'dist[flavour=min,target=roblox]'   # an exact task identifier
pcmp build dist --axis target=roblox           # by coordinate, repeatable
```

There is no wildcard. `--axis` says what a glob would have said, cannot match something by
accident, and needs no dialect to explain. A selection that matches nothing is an error
rather than a quiet success.

Because a task identifier is `profile[axis=value,…]`, a profile name cannot contain `[`,
`]`, `,` or `=`. That is `bad-name`.

## Reading a build

```
$ pcmp build
  built   dev      dist/dev/app.luau      (12 ms)
  cached  release  dist/release/app.luau  (0 ms)

1 built, 1 cached, 0 failed
```

Tasks run in parallel. A failure is collected rather than aborted on, so one run names
every task that went wrong.

```
$ pcmp plan --why
  built  dev  dist/dev/app.luau  — a source file changed
```

## Machine-readable output

`--json` is byte-identical for the same build, so it can be diffed. Durations are left out
for that reason; `--timings` puts them back.

```sh
pcmp build --json | jq '.tasks[] | select(.status == "failed")'
pcmp check --json | jq '[.[] | select(.severity == "error")] | length'
```

Every diagnostic has the same shape wherever it appears:

```json
{ "code": "…", "severity": "…", "at": "…", "message": "…", "help": "…", "source": null }
```

`at` is a manifest key path such as `profiles.release.darklua.rules[2]`. It is not a line
number, because a value a Luau manifest computed does not have one.

Findings that are the command's answer go to stdout; findings that mean the command failed
go to stderr, so `pcmp build > report.json` still tells you why it did not work. With
`--json`, both go to stdout and the exit code says which happened.

## Exit codes

| | |
| --- | --- |
| `0` | success |
| `1` | a task failed, or a `--frozen` build did not reproduce |
| `2` | the manifest could not be loaded or resolved |
| `3` | linting failed |

## In CI

```yaml
- run: pcmp check --strict
- run: pcmp build  --var version=${{ github.ref_name }}
- run: pcmp build  --frozen
```

`--frozen` replaces the old two-pass `verify`: it rebuilds from `pcmp.lock` and proves the
artifacts match, which is a stronger check and works when a manifest reads the clock.

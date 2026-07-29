---
description: How a build stays reproducible when its inputs do not.
---

# Determinism

A build is a function of what the manifest declares. The problem is that manifests
legitimately want things that change: a version, a git SHA, the time. ProCMP does not
forbid those — it records them.

## The ledger

Every value a manifest takes from outside itself is written down:

| | |
| --- | --- |
| `pcmp.env(name)` | an environment variable |
| `pcmp.now()` | an RFC 3339 instant in UTC |
| `pcmp.read(path)` | a file, recorded by digest |
| `--var`, `--define` | what you passed on the command line |

That set is the **ledger**. `pcmp build --lock` writes it to `pcmp.lock` along with a
digest of everything the build produced.

```sh
pcmp build --lock
```

```json
{
  "ledger": { "clock": "2026-07-29T00:13:04Z" },
  "tasks": { "release": { "plan": "…", "artifacts": "…" } }
}
```

## Reproducing

```sh
pcmp build --frozen
```

`--frozen` answers the manifest's questions from the lock instead of from the outside, so
`pcmp.now()` returns the instant the lock pinned. It then rebuilds with the cache off and
fails if any artifact differs.

This works with a timestamp in the header, which is the point. The timestamp is an input
like any other, and the lock is what makes it one you can reproduce.

A file read with `pcmp.read` is checked rather than replayed: the lock holds its digest,
and a frozen build reads the file and fails if it has changed. Resurrecting a stale copy
would be worse than saying so.

`SOURCE_DATE_EPOCH` is honoured when set, and `--now` overrides both.

## When nothing records it

```
$ pcmp check
warn[unrecorded-reading]: this manifest reads the clock or the environment,
  and nothing records it
  help: run `pcmp build --lock`; `pcmp build --frozen` then reproduces it exactly
```

Commit `pcmp.lock` and the warning goes away. A diff of it in review shows exactly what
changed about a build.

## What decides a rebuild

Four digests, and a task is skipped only when all four match:

| | |
| --- | --- |
| **plan** | the resolved task — paths, defines, rules, everything | 
| **shape** | every file path under your roots, without reading any of them |
| **reads** | the contents of the files darklua actually opened |
| **artifacts** | the files on disk, so an edited artifact is noticed |

Shape and reads are separate because a build can depend on a file's *absence*: if
`require("./mod")` resolves to `mod.luau` and you add `mod/init.luau`, nothing you already
had changed, but the answer did. Shape catches that without opening a file, which is also
why a no-op rebuild does not care how large your repository is.

`pcmp plan --why` names the one that moved.

## What counts as an input

Every file under the manifest's directory, plus every `sources` root, minus anything
`ignore` matches, minus every task's output and the cache directory.

```json5
sources: ["../shared"],      // extra roots
ignore: ["**/Packages/**"],  // wax globs, relative to each root
```

Extension is never a filter, because a loader can make a `.json` or a `.png` a real input.

A build reads only what it staged, so a file outside every root cannot be opened at all.
That is deliberate: an undeclared dependency becomes `undeclared-input`, naming the exact
path, instead of quietly deciding your output.

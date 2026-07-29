---
description: How a build stays reproducible when its inputs do not.
icon: fingerprint
---

# Determinism

A build is a function of what the manifest declares. The awkward part is that manifests legitimately want things that change, like a version, a git SHA, or the time.

ProCMP does not forbid those. It records them.

## The ledger

Every value a manifest takes from outside itself is written down.

| | |
| --- | --- |
| `pcmp.env(name)` | an environment variable |
| `pcmp.now()` | an RFC 3339 instant in UTC |
| `pcmp.epoch()` | seconds, consistent with `now()` inside one run |
| `pcmp.read(path)` | a file, recorded by digest |
| `--var`, `--define` | what you passed on the command line |

That set is the **ledger**.

```sh
pcmp build --lock
```

{% code title="pcmp.lock" %}
```json
{
  "version": 1,
  "ledger": { "clock": "2026-07-29T00:13:04Z" },
  "tasks": {
    "release": {
      "plan": "3f0c8a1e4b7d92a6c5e8f0b3d7a1c4e69b2f8d05a3c7e1b4f6098d2a5c3e7b91",
      "artifacts": "8b4e2c9f7a1d5306e8b3f0c7a2d94e15b6c8f3a0d7e2b591c4f60a8d3b25e7c9"
    }
  }
}
```
{% endcode %}

Commit it. A diff of this file in review shows exactly what changed about a build.

## Reproducing

```sh
pcmp build --frozen
```

`--frozen` answers the manifest's questions from the lock instead of from the outside, so `pcmp.now()` returns the instant the lock pinned. It then rebuilds with the cache off and fails if any artifact differs.

{% hint style="success" %}
This is why a build timestamp in a header costs you nothing.

The timestamp is an input like any other, and the lock is what makes it one you can reproduce.
{% endhint %}

A file read with `pcmp.read` is checked rather than replayed. The lock holds its digest, a frozen build reads the file, and a mismatch is a failure. Resurrecting a stale copy would be worse than saying so.

`SOURCE_DATE_EPOCH` is honoured when set, and `--now` overrides both.

## When nothing records it

```
$ pcmp check
warn[unrecorded-reading]: this manifest reads the clock or the environment, and nothing records it
  help: run `pcmp build --lock`, and `pcmp build --frozen` then reproduces it exactly
```

The warning goes away once a lock exists.

## What decides a rebuild

Four digests, and a task is skipped only when all four match.

| Digest | Covers | Answers |
| --- | --- | --- |
| `plan` | the resolved task, meaning paths, defines, rules, everything | is this the same build? |
| `shape` | every file path under your roots, without reading one | did a file appear, vanish or move? |
| `reads` | the contents of the files darklua actually opened | did a source change? |
| `artifacts` | the files on disk | is what is there what we wrote? |

`pcmp plan --why` names the one that moved.

### Why shape and reads are separate

{% columns %}
{% column %}
**shape** answers a question about names.

It walks every root and records each path with its kind, and reads no file at all.

A symlink carries its target, because retargeting one changes a build without changing a file.
{% endcolumn %}

{% column %}
**reads** answers a question about contents.

It opens only the files darklua actually followed, which after the first build is a handful.

Edit a file outside that set and neither digest moves, which is correct.
{% endcolumn %}
{% endcolumns %}

A build can depend on a file's *absence*. If `require("./mod")` resolves to `mod.luau` and you add `mod/init.luau`, nothing you already had changed, but the answer did. Only shape sees that, and it sees it without opening anything, which is also why a no-op rebuild does not care how large your repository is.

The `artifacts` digest is the one that notices an artifact edited by hand, which an inputs-only stamp never could.

## What counts as an input

Every file under the manifest's directory, plus every `sources` root. Minus anything `ignore` matches, minus every task's output, minus the cache directory.

```json5
sources: ["../shared"],
ignore: ["**/Packages/**"],
```

Extension is never a filter, because a loader can make a `.json` or a `.png` a real input.

{% hint style="danger" %}
A build reads only what it staged, so a file outside every root cannot be opened at all.

That is deliberate. An undeclared dependency becomes [`undeclared-input`](diagnostics.md#undeclared-input), naming the exact path, instead of quietly deciding your output.
{% endhint %}

## Two files, two jobs

| | |
| --- | --- |
| `.pcmp/` | local cache, gitignored, disposable, one record per task |
| `pcmp.lock` | committed provenance, and what `--frozen` reproduces |

The lock is written only by `--lock`. A manifest calling `pcmp.now()` would otherwise rewrite it on every invocation, which is the same mistake in a new place.

<details>

<summary>What a cache record holds</summary>

One file per task under `.pcmp/`, named by the digest of the task identifier, because an identifier can contain `[`, `]`, `=` and `,`.

```json
{
  "version": 1,
  "plan": "3f0c8a1e4b7d92a6",
  "shape": "8b4e2c9f7a1d5306",
  "reads": "a17d05e3c8b40f92",
  "artifacts": "c93f21a7e05b8d34",
  "darklua": "0.19.0",
  "read_set": ["src/init.luau", "src/gen/data.luau"],
  "outputs": ["dist/release/app.luau"]
}
```

`read_set` is what the next build hashes instead of walking everything.

`outputs` is what a later build is allowed to remove when it stops producing them. ProCMP never deletes a file it did not write.

</details>

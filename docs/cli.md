---
description: Selecting tasks, reading a build, and what an exit code means.
icon: terminal
---

# CLI

`pcmp help` lists every command and flag. `pcmp explain <CODE>` describes any diagnostic you see, and [Diagnostics](diagnostics.md) is the same catalogue as a page.

None of that is repeated here, so none of it can go stale. This page covers what those cannot.

## Selecting tasks

`build` and `watch` take a selection. Without one they take everything.

```sh
pcmp build release
pcmp build 'dist[flavour=min,target=roblox]'
pcmp build dist --axis target=roblox
```

A selector is a profile name or an exact task identifier. `--axis KEY=VALUE` filters an expansion by coordinate, and repeats.

{% hint style="info" %}
There is no wildcard.

`--axis` says what a glob would have said, cannot match something by accident, and needs no dialect to explain. A selection that matches nothing is an error rather than a quiet success.

Because a task identifier is `profile[axis=value]`, a profile name cannot contain `[`, `]`, `,` or `=`. That is `bad-name`.
{% endhint %}

## Reading a build

```
$ pcmp build
plan  9b35cd36a0d9

  built   dev      dist/dev/app.luau      12 ms
  cached  release  dist/release/app.luau  0 ms

1 built, 1 cached, 0 failed
```

Every screen opens with the digest of the plan it resolved, lists one row per task, and closes with a count of what it just showed. `plan`, `build` and `watch` all read the same way for that reason.

Tasks run in parallel. A failure is collected rather than aborted on, so one run names every task that went wrong.

A task is skipped when its configuration, its sources and its artifacts are all unchanged. `--why` names whichever of those moved.

```
$ pcmp plan --why
plan  9b35cd36a0d9

  stale   dev      dist/dev/app.luau      a source file changed
  fresh   release  dist/release/app.luau

1 stale, 1 fresh, 0 failed
```

`plan --why` builds nothing, so it says `stale` and `fresh` where a build says `built` and `cached`.

`pcmp plan <TASK>` prints everything one task resolved to, including the darklua configuration it compiles to.

## Reproducing a build

A manifest may read the clock, an environment variable, or a file.

```lua
vars = { version = pcmp.envOr("VERSION", "dev"), built = pcmp.now() }
```

Those change between runs, so `pcmp` writes down what it read.

```sh
pcmp build --lock      # build, and record what it read
pcmp build --frozen    # build from that record, and fail if anything differs
```

`--frozen` answers the manifest from `pcmp.lock` instead of from the outside, so `pcmp.now()` returns the instant the lock pinned. That is why a build timestamp costs you nothing.

Commit `pcmp.lock`. A diff of it in review shows exactly what changed about a build.

Ignore `.pcmp/`, which is the local build cache and is disposable.

### Pinning the clock

Whatever `pcmp.now()` returns lands in the resolved task, and the plan digest covers the task, so a manifest that calls it resolves to a different task every second and an ordinary build never hits the cache. Pin the clock and caching comes back.

{% tabs %}
{% tab title="--now" %}
```sh
pcmp build --now 2026-01-01T00:00:00Z
```
{% endtab %}

{% tab title="SOURCE_DATE_EPOCH" %}
```sh
SOURCE_DATE_EPOCH=1767225600 pcmp build
```
{% endtab %}
{% endtabs %}

The two are the same instant written two ways. `--now` takes an RFC 3339 instant in UTC to the second and beats `SOURCE_DATE_EPOCH`, which takes seconds since the Unix epoch and is the convention the rest of a release pipeline already reads. A frozen build beats both, because the lock is the point of freezing.

{% hint style="info" %}
`pcmp check` reports [`unrecorded-reading`](diagnostics.md#unrecorded-reading) when a manifest reads something and no lock records it.

The warning goes away once a lock exists.
{% endhint %}

## Machine-readable output

`--json` is byte-identical for the same build, so it can be diffed. Durations are left out for that reason, and `--timings` puts them back.

```sh
pcmp build --json | jq '.tasks[] | select(.status == "failed")'
pcmp check --json | jq '[.[] | select(.severity == "error")] | length'
```

Every diagnostic has the same shape wherever it appears.

<details>

<summary>The diagnostic shape</summary>

```json
{
  "code": "missing-output",
  "severity": "error",
  "at": "profiles.release",
  "message": "no `output` after inheritance",
  "help": "a template, e.g. \"dist/{profile}/app.luau\"",
  "source": null
}
```

| Field | |
| --- | --- |
| `code` | stable, and never reused for another meaning. See [Diagnostics](diagnostics.md) |
| `severity` | `error` or `warning` |
| `at` | a manifest key path such as `profiles.release.darklua.rules[2]` |
| `message` | one line, lowercase, no trailing stop |
| `help` | zero or more lines of what to do |
| `source` | the underlying operating system error, when there was one |

`at` is a key path rather than a line number, because a value a Luau manifest computed does not have one.

</details>

{% hint style="info" %}
Findings that are the command's answer go to stdout. Findings that mean the command failed go to stderr, so `pcmp build > report.json` still tells you why it did not work.

With `--json` both go to stdout, because that is where machine output lives and the exit code already says which happened.
{% endhint %}

## Exit codes

| | |
| --- | --- |
| `0` | success |
| `1` | a task failed, or a `--frozen` build did not reproduce |
| `2` | the manifest could not be loaded or resolved |
| `3` | linting failed |

## In CI

{% code title=".github/workflows/build.yml" %}
```yaml
- run: pcmp check --strict
- run: pcmp build  --var version=${{ github.ref_name }}
- run: pcmp build  --frozen
```
{% endcode %}

`--frozen` rebuilds from `pcmp.lock` and proves the artifacts match. It is a stronger check than building twice and comparing, and unlike that check it still works when a manifest reads the clock.

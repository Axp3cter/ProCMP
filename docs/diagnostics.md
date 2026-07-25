---
description: Every code pcmp check reports.
---

# Diagnostics

Codes are stable names, never reused for another meaning. Four mistakes report four
findings in one run, and `--strict` makes warnings fail too.

```
$ pcmp check
error  missing-output: task `release` has no `output`
       help: set `output`, e.g. "dist/{profile}/app.luau"
warn   unused-profile: abstract profile `old` is never extended or used as a matrix base
       help: remove it, or drop `abstract` so it builds

1 error(s), 1 warning(s)
```

## Resolution

Always errors. Nothing is built.

| Code | Meaning |
| --- | --- |
| `unknown-base` | `extends` names a profile that does not exist |
| `cyclic-extends` | An `extends` chain forms a cycle |
| `missing-entry` | No `entry` after inheritance |
| `missing-output` | No `output` after inheritance |
| `bad-template` | An `output` or `header` token is unknown, empty, or unclosed |
| `bad-path` | A path in `entry`, `output` or `sources` is empty, or escapes the root |
| `unknown-matrix-base` | A matrix `base` names a profile that does not exist |
| `empty-axis` | A matrix axis has no values |
| `duplicate-axis-value` | A matrix axis lists the same value twice |
| `bad-define` | A define key is not a Luau identifier, or its value is not finite |
| `bad-var` | A var name is not a name, or two collide as one constant |
| `bad-rules` | `darklua.rules` is not a list darklua could read |
| `darklua-loaders` | Loaders declared in both `loaders` and `darklua.loaders` |
| `no-tasks` | No profiles and no matrix |

{% hint style="warning" %}
`inject_global_value` substitutes by name, so a define key such as `my-flag` or `end`
would match nothing. A var becomes `PCMP_<NAME>` uppercased, so `channel` and `Channel`
collide.
{% endhint %}

## Rule order

Errors. Both are orderings darklua's own documentation calls out.

| Code | Meaning |
| --- | --- |
| `fold-before-inject` | `compute_expression` scheduled before `inject_global_value` |
| `branch-before-fold` | `remove_unused_if_branch` scheduled before `compute_expression` |

Nothing beyond these two. darklua's own default rule list is a valid ordering, and a
lint stricter than the tool it lints for would reject working manifests.

## Hygiene

Warnings. The plan still builds.

| Code | Meaning |
| --- | --- |
| `unused-profile` | An `abstract` profile nothing extends |
| `identical-profiles` | Two profiles declared identically |

## Errors without a code

Reported on stderr and not part of `check`: two tasks claiming one output, a missing
entry point, a manifest that does not parse, an `ignore` entry that is not a valid glob,
a configuration darklua rejected, and a selection that matched nothing.

```
$ pcmp build nosuch
error: no task matched `nosuch`
  known tasks: dev, release
```

## In CI

```sh
pcmp check --strict
pcmp check --json | jq '[.[] | select(.severity == "deny")] | length'
```

Exit code `5` means linting failed. See [Exit codes](cli.md#exit-codes).

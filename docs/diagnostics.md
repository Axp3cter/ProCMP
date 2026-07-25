---
description: What pcmp check reports.
---

# Diagnostics

```
$ pcmp check
error  missing-output: task `release` has no `output`
       help: set `output`, e.g. "dist/{profile}/app.luau"
warn   unused-profile: abstract profile `old` is never extended or used as a matrix base
       help: remove it, or drop `abstract` so it builds

1 error(s), 1 warning(s)
```

Codes are stable names. `--strict` makes warnings fail too.

## Resolution

Always errors. Nothing is built.

```
unknown-base          extends names a profile that does not exist
cyclic-extends        an extends chain forms a cycle
missing-entry         no entry after inheritance
missing-output        no output after inheritance
bad-template          an output or header token is unknown, empty, or unclosed
bad-path              a path in entry, output or sources is empty, or escapes the root
unknown-matrix-base   matrix base names a profile that does not exist
empty-axis            a matrix axis has no values
duplicate-axis-value  a matrix axis lists the same value twice
bad-define            a define key is not a Luau identifier, or its value is not finite
bad-var               a var name is not a name, or two collide as one constant
bad-rules             darklua.rules is not a list darklua could read
darklua-loaders       loaders declared in both `loaders` and `darklua.loaders`
no-tasks              no profiles and no matrix
```

`inject_global_value` substitutes by name, so a define key such as `my-flag` or `end`
matches nothing. A var becomes `PCMP_<NAME>` uppercased, so `channel` and `Channel`
collide.

## Rule order

Errors. Both are orderings darklua's own documentation calls out.

```
fold-before-inject    compute_expression scheduled before inject_global_value
branch-before-fold    remove_unused_if_branch scheduled before compute_expression
```

## Hygiene

Warnings. The plan still builds.

```
unused-profile        an abstract profile nothing extends
identical-profiles    two profiles declared identically
```

## Errors without a code

Reported on stderr: two tasks claiming one output, a missing entry point, a manifest
that does not parse, an `ignore` entry that is not a valid glob, a configuration darklua
rejected, and a selection that matched nothing.

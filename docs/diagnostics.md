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

Codes are stable names. Findings accumulate, so four mistakes report four in one run.
`--strict` makes warnings fail too.

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
bad-define            a define key is not a Luau identifier, or its value is not finite
bad-var               a var name cannot become a token and a constant
bad-rules             darklua.rules is not a list darklua could read
darklua-loaders       loaders declared in both `loaders` and `darklua.loaders`
no-tasks              no profiles and no matrix
```

`bad-define` covers both halves of "this define cannot become a literal".
`inject_global_value` substitutes by name, so a key such as `my-flag` or `end` matches
nothing. Infinity and NaN have no literal form. A var and a matrix axis each contribute
a `PCMP_<NAME>` constant, so their names are held to the same rule.

`darklua-loaders` exists because only one of the two can win, and picking silently would
mean the manifest format decided which loader pattern applied. See
[Manifest](manifest.md#loaders).

## Rule order

Errors. Both are orderings darklua's own documentation calls out.

```
fold-before-inject    compute_expression scheduled before inject_global_value
branch-before-fold    remove_unused_if_branch scheduled before compute_expression
```

The order is yours. These report rather than rewrite, because a silently reordered
pipeline would make `pcmp explain` a lie. See [darklua](darklua.md).

## Hygiene

Warnings. The plan still builds.

```
unused-profile        an abstract profile nothing extends
identical-profiles    two profiles declared identically
```

## Errors without a code

Reported on stderr: two tasks claiming one output path, a missing entry point, a
manifest that does not parse, an `ignore` entry that is not a valid glob, a configuration
darklua rejected, which carries the JSON that was emitted, and a task filter that
matched nothing.

## Exit codes

<table><thead><tr><th width="90">Code</th><th></th></tr></thead><tbody>
<tr><td><code>0</code></td><td>Success.</td></tr>
<tr><td><code>1</code></td><td>A build task failed, or output was not reproducible.</td></tr>
<tr><td><code>2</code></td><td>The manifest could not be loaded or resolved.</td></tr>
<tr><td><code>5</code></td><td><code>check</code> found an error, or a warning under <code>--strict</code>.</td></tr>
</tbody></table>

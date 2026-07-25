---
description: What counts as a build input, and how the cache knows.
---

# Inputs

A task is skipped when nothing it reads has changed. The set of files that answer
depends on is derived from the plan, not guessed at from directory names.

## What is hashed

Every file under every root, whatever its extension.

<table><thead><tr><th width="200">Root</th><th></th></tr></thead><tbody>
<tr><td>The manifest's directory</td><td>Always</td></tr>
<tr><td><code>sources</code></td><td>Anything else the build reads</td></tr>
</tbody></table>

There is no extension allowlist. A content [loader](manifest.md#loaders) can make a
`.json` or a `.png` a real build input.

## What is not

<table><thead><tr><th width="200"></th><th></th></tr></thead><tbody>
<tr><td>Every task's <code>output</code></td><td>Taken from the plan. A build that hashed its own artifacts would invalidate the next one</td></tr>
<tr><td>The cache directory</td><td><code>.pcmp/</code>, or wherever <code>--cache-dir</code> points</td></tr>
<tr><td><code>.git</code></td><td>Never an input, and it changes on every commit</td></tr>
<tr><td><code>ignore</code></td><td>Yours</td></tr>
</tbody></table>

Nothing else is assumed. `dist`, `build`, `node_modules` and `target` are not special
names. If one of them is where your `output` goes it is excluded because it is your
output, and if it is where your source lives it is hashed because it is your source.

## Reaching outside the project

```lua
base = {
	entry   = "src/init.luau",
	output  = "dist/app.luau",
	sources = { "../shared", "../protocol" },
},
```

Editing `../shared` invalidates the cache and wakes `pcmp watch`. A root nested inside
another is folded into the outer one.

## Skipping a vendored tree

```lua
ignore = { "**/Packages/**", "**/*.rbxm" },
```

[wax glob syntax](https://github.com/olson-sean-k/wax#patterns), matched against paths
relative to each root. This is the same dialect darklua uses for `apply_to_files`,
`skip_files` and loader patterns.

This is a speed control, not a correctness one. Ignoring a directory the build actually
reads means edits there will not rebuild.

## Watching

`pcmp watch` watches exactly this set, so what wakes it and what invalidates the cache
cannot drift apart. The manifest itself is watched too, and re-read every cycle.

```sh
pcmp watch release
pcmp watch '*target=roblox*'
```

## Cache location

State lives in `.pcmp/` beside the manifest. Add it to `.gitignore`, or move it:

```sh
pcmp build --cache-dir "$RUNNER_TEMP/pcmp"
```

Useful for a read-only checkout, or for sharing one cache across several worktrees.

## What the key covers

Resolved configuration, the fingerprint of every input above, and the darklua version
linked into the binary. The last matters because the same manifest can produce different
bytes against a different darklua.

Editing one file rebuilds every task. darklua follows requires, so a per-task input list
would have to guess at what a module pulls in.

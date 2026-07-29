---
title: Manifest
description: Every field, and what it does.
---

# Manifest

Discovery starts in the working directory and walks up, trying each name in turn.

```
pcmp.json5   pcmp.json   pcmp.jsonc   pcmp.toml   pcmp.luau
```

Format comes from the extension, never from the content. Relative paths always resolve against the manifest's own directory, not the working directory.

All five formats resolve to the same plan, digest included. That digest does not depend on where the project is checked out or on how the manifest is arranged, only on what it means.

=== "pcmp.json5"

    ```json5
    {
      vars: { name: "app" },
      profiles: {
        release: {
          entry: "src/init.luau",
          output: "dist/{name}.luau",
          define: { DEBUG: false },
          darklua: { generator: "dense", rules: ["compute_expression"] },
        },
      },
    }
    ```

    Comments, trailing commas and unquoted keys. `pcmp.json` and `pcmp.jsonc` go through the same parser.

=== "pcmp.toml"

    ```toml
    [vars]
    name = "app"

    [profiles.release]
    entry  = "src/init.luau"
    output = "dist/{name}.luau"
    define = { DEBUG = false }

    [profiles.release.darklua]
    generator = "dense"
    rules     = ["compute_expression"]
    ```

=== "pcmp.luau"

    ```lua
    return {
    	vars = { name = "app" },
    	profiles = {
    		release = {
    			entry   = "src/init.luau",
    			output  = "dist/{name}.luau",
    			define  = { DEBUG = false },
    			darklua = { generator = "dense", rules = { "compute_expression" } },
    		},
    	},
    }
    ```

    Only this format can compute a value rather than be given one, through [the pcmp API](#the-pcmp-api).

## Top level

| Key | What it holds |
| --- | --- |
| `vars` | named values every profile starts from |
| `templates` | never built, and exist to be extended |
| `profiles` | built, once each, or once per axis combination |

A name may appear in `templates` or in `profiles`, not both. `extends` looks in one namespace so that it needs no precedence rule, and a name in both is [`name-collision`](diagnostics.md#name-collision).

## Profile fields

| Field | What it does |
| --- | --- |
| `extends` | a template or profile to inherit from |
| `entry` | a file to bundle, or a directory to process as a tree |
| `output` | where it goes |
| `sources` | extra files and directories that count as build inputs |
| `ignore` | globs excluded from that input set |
| `vars` | named values, each becoming a `{token}` and a `PCMP_<NAME>` constant |
| `define` | constants substituted into your source |
| `header` | lines written above each artifact, after darklua discards comments |
| `loaders` | an ordered list of `pattern` to `use` pairs |
| `darklua` | [darklua's own configuration](darklua.md), verbatim |
| `axes` | expands the profile into one task per combination |

Everything except `vars` and `define` replaces wholesale down an `extends` chain. Those two accumulate, and `darklua` merges key by key.

## Templates

`entry`, `output`, `sources` and `header` are templates. `{profile}`, every var and every axis expand, `{{` and `}}` are literal braces, and an unknown token is [`bad-template`](diagnostics.md#bad-template) rather than an empty string.

In a path, a token may not expand to a `.` or `..` segment. `dist/{profile}/app.luau` under a profile named `..` would write `app.luau`, outside the directory the template names, so it is refused. A plain `/` is allowed, and `{outdir}/app.luau` with `outdir` set to `build/dist` works.

## Vars and defines

A var names a value used to build a path, a header, or a constant. A define names a value substituted into your source. Every var is also a define, uppercased and prefixed. The reverse does not hold, because a define needs no name that is also a path token.

```json5
vars:   { name: "app", retries: 3 },
define: { DEBUG: false }
```

Both take a string, a number or a boolean. The type survives into the emitted Luau, so `retries: 3` is a number your source can do arithmetic on, and it reaches the cache key type-tagged, which is why `true` and `"true"` are different builds.

An integer must survive a round trip through an IEEE double, which bounds it at 2^53. Past that is [`bad-define`](diagnostics.md#bad-define), not a silent change of value.

## Entry and output

=== "File to file"

    ```json5
    entry: "src/init.luau", output: "dist/app.luau",
    ```

    Bundled into one file when `darklua.bundle` is set.

=== "Directory to directory"

    ```json5
    entry: "src", output: "build",
    ```

    Every file processed, structure preserved, no bundling.

`header` applies to whatever darklua emits as source, meaning `.luau` and `.lua`, or whatever `darklua.lua_extension` says. A `copy` loader can put a `.png` in the output tree, and a `.png` with a Lua comment on the front is a broken `.png`.

Two tasks writing one path is [`output-collision`](diagnostics.md#output-collision), and a task writing inside an entry tree is [`output-in-inputs`](diagnostics.md#output-in-inputs). An `output` that climbs out of the project with `..` is legal, and reported as [`output-outside-root`](diagnostics.md#output-outside-root) because it means `pcmp` is writing where nobody reading the manifest expects it to.

## Inputs

A build reads every file under the manifest's directory, plus every `sources` root, minus anything `ignore` matches. Outputs and the cache directory are never inputs, so you do not have to exclude them, and extension is never a filter either, because a loader can make a `.json` or a `.png` a real input.

That set is measured twice, and the difference is what keeps a large `sources` root cheap.

Shape

:   Which paths exist under every root, and what kind each one is. No file is opened. A file appearing, vanishing or being retargeted as a symlink moves this, which is how a build can depend on a file's absence.

Reads

:   The contents of the files darklua actually opened. Editing a file nothing requires changes neither digest, so it costs nothing.

`pcmp plan --why` names whichever of the two moved.

```console
$ pcmp plan --why
  stale   release  dist/release/app.luau  a file appeared, vanished or moved
```

!!! warning "A build can only open what the manifest declares"

    Sources are staged in memory before darklua runs, so a file outside every root cannot be reached at all. The build fails with [`undeclared-input`](diagnostics.md#undeclared-input) naming the exact path, rather than quietly deciding your output.

    Add the directory that holds it to `sources`.

## Loaders

Teaches darklua what to do with files it would otherwise ignore.

```json5
loaders: [
  { pattern: "**/*.png", use: "buffer/base64" },
  { pattern: "**/*.md", use: "string" },
]
```

Require them with the extension.

```lua
local config = require("@self/assets/config.json")
```

| `use` | Behaviour |
| --- | --- |
| `copy` | passed through untouched |
| `skip` | excluded from the output |
| `luau` | parsed and processed as source |
| `json`, `json_lines`, `toml`, `yaml` | returned as parsed data |
| `string`, `buffer`, `bytes` | returned as content |

The content forms also take an encoding, one of `/base64`, `/zstd`, `/gzip` or `/zlib`.

darklua takes the first pattern that matches. That is why `loaders` is a list in every format and never a map keyed by pattern: only a list has an order, and a map that happened to have one would make the winner depend on how the manifest was written.

## Axes

```json5
axes: {
  flavour: ["min", "dev"],
  target: {
    roblox: { darklua: { bundle: { require_mode: "path" } } },
    lune:   { darklua: { bundle: { require_mode: "luau" } } },
  },
}
```

An axis is a list of values, or a map from a value to an overlay that can set any profile field. Overlays apply in axis-name order, so two axes touching the same key resolve predictably.

Each combination becomes a task named by its coordinates, and each axis is also a var.

## The pcmp API

Luau manifests only, and the manifest's only channel outward. Everything here is recorded, which is what makes it safe to use. See [Reproducing a build](cli.md#reproducing-a-build).

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
pcmp.now()                        -- RFC 3339 UTC
pcmp.epoch()                      -- seconds, consistent with now() in one run
pcmp.read("VERSION")              -- a file, relative to the manifest
pcmp.root                         -- the manifest's directory
pcmp.darklua                      -- the linked darklua version
```

`pcmp.now()` answers differently every second and its answer lands in the resolved task, so a manifest that calls it rebuilds on every run. [Pinning the clock](cli.md#pinning-the-clock) gives the cache back while you work.

There is deliberately no `pcmp.exec`. Recording a subprocess honestly would mean hashing its whole environment, and a ledger that lies is worse than no ledger. Pass a git SHA in with `--var`.

??? note "What the sandbox removes"

    Luau's own sandbox takes `os`, `io`, `load`, `dofile`, `debug` and `coroutine`.

    `pcmp` additionally revokes `collectgarbage`, `getfenv`, `loadstring`, `newproxy`, `require` and `setfenv`, plus `math.random` and `math.randomseed`. Random is the one impurity the ledger cannot capture, because there is nothing to write down that would reproduce it.

    Evaluation is bounded at 50 million VM steps and 32 MB, and exceeding either is [`budget`](diagnostics.md#budget). `print` is redirected to stderr, so a manifest that prints cannot land in the middle of `--json`.

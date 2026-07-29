---
description: Every field, and what it does.
---

# Manifest

Discovery starts in the working directory and walks up, trying each name in turn:

```
pcmp.json5   pcmp.json   pcmp.jsonc   pcmp.toml   pcmp.luau
```

Format comes from the extension, never from the content. Relative paths always resolve
against the manifest's own directory, not the working directory.

All five formats resolve to the same plan, digest included — and the digest does not
depend on where the project is checked out or on how the manifest is arranged, only on
what it means.

## Top level

| | |
| --- | --- |
| `vars` | named values every profile starts from |
| `templates` | never built; exist to be extended |
| `profiles` | built, once each, or once per axis combination |

A name may appear in `templates` or in `profiles`, not both: `extends` looks in one
namespace so that it needs no precedence rule.

## Profile fields

| | |
| --- | --- |
| `extends` | a template or profile to inherit from |
| `entry` | a file to bundle, or a directory to process as a tree |
| `output` | where it goes |
| `sources` | extra files and directories that count as build inputs |
| `ignore` | globs excluded from that input set |
| `vars` | named values. Each becomes a `{token}` and a `PCMP_<NAME>` constant |
| `define` | constants substituted into your source |
| `header` | lines written above each artifact, after darklua discards comments |
| `loaders` | ordered `pattern` to `use` pairs |
| `darklua` | [darklua's own configuration](darklua.md), verbatim |
| `axes` | expands the profile into one task per combination |

`entry`, `output` and `header` are templates: `{profile}`, every var and every axis
expand, `{{` and `}}` are literal braces, and an unknown token is an error rather than an
empty string.

## Vars and defines

A var names a value used to build a path, a header, or a constant. A define names a value
substituted into your source. Every var is also a define, uppercased and prefixed; the
reverse does not hold, because a define needs no name that is also a path token.

```json5
vars:   { name: "app", retries: 3 },   // {name}, {retries}, PCMP_NAME, PCMP_RETRIES
define: { DEBUG: false }               // DEBUG
```

Both take a string, a number or a boolean. The type survives into the emitted Luau, so
`retries: 3` is a number your source can do arithmetic on, and it reaches the cache key
type-tagged — `true` and `"true"` are different builds.

Integers must survive a round trip through a double, which bounds them at 2^53. Past that
is `bad-define`, not a silent change of value.

`_G.DEBUG` and `_G["DEBUG"]` work identically. `getgenv().DEBUG` does not: it is a function
call, so there is nothing to replace at build time.

## Values from outside

Three ways in, in order of precedence:

```sh
pcmp build --var version=v1.2.3 -D CHANNEL=beta
```

```lua
vars = { version = pcmp.env("VERSION") },                  -- errors when unset
vars = { version = pcmp.envOr("VERSION", "v0.0.0-dev") },  -- explicit fallback
vars = { built   = pcmp.now() },                           -- recorded, see determinism.md
```

`--var` and `--define` work whatever the format and beat the manifest.

## Entry and output

`entry: "src/init.luau"` with `output: "dist/app.luau"` bundles one file into one file, if
`darklua.bundle` is set. `entry: "src"` with `output: "build"` processes every file and
preserves the structure, without bundling.

`header` applies to whatever darklua emits as source — `.luau` and `.lua`, or whatever
`darklua.lua_extension` says.

Two tasks writing one path is `output-collision`. A task writing inside another's entry
tree is `output-in-inputs`.

## Loaders

Teaches darklua what to do with files it would otherwise ignore:

```json5
loaders: [
  { pattern: "**/*.png", use: "buffer/base64" },
  { pattern: "**/*.md", use: "string" },
]
```

Require them with the extension: `require("@self/assets/config.json")`.

| `use` | |
| --- | --- |
| `copy` | passed through untouched |
| `skip` | excluded from the output |
| `luau` | parsed and processed as source |
| `json`, `json_lines`, `toml`, `yaml` | returned as parsed data |
| `string`, `buffer`, `bytes` | returned as content |

The content forms also take an encoding: `/base64`, `/zstd`, `/gzip` or `/zlib`.

darklua takes the first pattern that matches, so order decides. A list always has one; a
map has one only in a data format, so the map spelling is refused from a Luau manifest,
where a table does not.

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

An axis is a list of values, or a map from a value to an overlay that can set any profile
field. Overlays apply in axis-name order, so two axes touching the same key resolve
predictably.

Each combination becomes a task named by its coordinates, and each axis is also a var.

## The `pcmp` API

Luau manifests only, and the manifest's only channel outward. Every entry records what it
answered with — see [Determinism](determinism.md).

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
pcmp.now()                        -- RFC 3339 UTC
pcmp.epoch()                      -- seconds, consistent with now() within a run
pcmp.read("VERSION")              -- a file, relative to the manifest
pcmp.root                         -- the manifest's directory
pcmp.darklua                      -- the linked darklua version
```

There is deliberately no `pcmp.exec`. Recording a subprocess honestly would mean hashing
its whole environment, and a ledger that lies is worse than no ledger. Pass a git SHA in
with `--var`.

The sandbox removes `os`, `io`, `load`, `require`, `debug`, `coroutine` and
`math.random`, and bounds evaluation by steps and memory.

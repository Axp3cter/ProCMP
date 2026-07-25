---
description: Every field, in one annotated manifest.
---

# Manifest

`pcmp.json5`, `pcmp.json`, `pcmp.jsonc`, `pcmp.toml` and `pcmp.luau` all resolve to the
same plan, and that is discovery order. Discovery starts in the working directory and
walks up. Relative paths always resolve against the manifest's own directory.

`pcmp init` writes JSON5: plain data any tool can rewrite, with `$schema` giving an
editor validation. Luau is the one format that can compute a value rather than be given
one, through [`pcmp.env`](#the-pcmp-api).

ProCMP owns eight profile fields. Everything that configures a transformation lives
under [`darklua`](darklua.md) and is darklua's own format.

{% code title="pcmp.json5" %}
```json5
{
  // Each becomes a {token} and a PCMP_<NAME> constant.
  vars: { name: "app", version: "v0.0.0-dev" },

  profiles: {
    base: {
      abstract: true,                      // never built, exempt from entry and output
      entry: "src/init.luau",              // a file, or a directory
      output: "dist/{profile}/{name}.luau",
      sources: ["../shared"],              // extra input roots
      ignore: ["**/Packages/**"],          // excluded from the input set

      darklua: {
        bundle: { require_mode: "luau" },
      },
    },

    release: {
      extends: "base",
      define: { DEBUG: false, RETRIES: 3 },

      // Written above each artifact after darklua runs, so it survives minification.
      header: ["--!native", "-- {name} {version}"],

      // Ordered: darklua takes the first pattern that matches.
      loaders: [
        { pattern: "**/*.model.json", use: "copy" },
        { pattern: "**/*.json", use: "json" },
      ],

      darklua: {
        generator: { name: "dense", column_span: 120 },
        rules: ["compute_expression", "remove_unused_if_branch", "rename_variables"],
      },
    },
  },

  matrix: {
    dist: {
      base: "base",
      axes: { target: ["roblox", "lune"] },
      output: "dist/{target}/{name}.luau",
      define: { BUNDLED: true },
    },
  },
}
```
{% endcode %}

## Vars and defines

A **var** names a string used to build a path or a header. A **define** names a value
substituted into your source.

```json5
vars: { name: "app", channel: "stable" },   // {name}, PCMP_NAME
define: { DEBUG: false, MAX_RETRY: 3 },     // DEBUG, MAX_RETRY
```

Every var is also a define. The reverse does not hold, because a define can be a boolean
or a number and neither belongs in a filename.

```lua
if DEBUG then
	print("verbose telemetry")
end

local channel: string = PCMP_CHANNEL
```

`_G.DEBUG` and `_G["DEBUG"]` work identically. `getgenv().DEBUG` does not: it is a
function call, so there is nothing to replace at build time.

A define is a boolean, a finite number, or a string. The type reaches the cache key, so
`true` and `"true"` are different builds. Infinity and NaN are `bad-define`.

## Values from outside

Three ways in, in order of precedence:

```sh
pcmp build --var version=v1.2.3 -D CHANNEL=beta
```

```lua
vars = { version = pcmp.env("VERSION") },                    -- errors when unset
vars = { version = pcmp.envOr("VERSION", "v0.0.0-dev") },    -- explicit fallback
```

`--var` and `--define` work whatever the format, so a JSON or TOML project is not stuck
with literals. There is no build-timestamp define: a timestamp makes two builds of the
same commit differ, which breaks `pcmp verify`. Pass one in if you want it.

## Inheritance

Nearer wins, field by field. `vars` and `define` accumulate, and `darklua` merges key by
key so a profile can set `generator` without restating `bundle`.

```json5
base:    { abstract: true, define: { A: 1, B: 1 }, header: ["--!native"] },
release: { extends: "base", define: { B: 2 } },
// release resolves to define { A: 1, B: 2 } and header ["--!native"]
```

Every list replaces outright, so a child clears one it inherited by declaring it empty:

```json5
plain: { extends: "base", header: [] },
```

## Entry and output

A file in, a file out, bundled if `darklua.bundle` is set:

```json5
entry: "src/init.luau", output: "dist/app.luau",
```

A directory in, a directory out. Every file processed, structure preserved, no bundling:

```json5
entry: "src", output: "build",
```

`header` applies to every `.luau` and `.lua` artifact either way, and `pcmp verify`
compares the whole tree.

`output` expands `{profile}`, every var, and every matrix axis. `{{` and `}}` are
literal braces. An unknown token is `bad-template`, not an empty string.

## Loaders

Teaches darklua what to do with files it would otherwise ignore, which is how you embed
assets:

```json5
loaders: [
  { pattern: "**/*.png", use: "buffer/base64" },
  { pattern: "**/*.md", use: "string" },
  { pattern: "**/*.json", use: "json" },
]
```

Require them **with the extension**, which is how darklua tells a data file from a
module:

```lua
local config = require("@self/assets/config.json")
```

Available: `copy`, `skip`, `json`, `json_lines`, `toml`, `yaml`, `string`, `buffer`,
`bytes`, and encoded forms such as `string/base64` and `buffer/zstd`.

An ordered list rather than darklua's map, because darklua takes the first pattern that
matches and a Luau table iterates in hash order. Declaring loaders in both places is
`darklua-loaders`, an error.

## Matrix

One task per combination, named by its coordinates.

```
$ pcmp plan
4 task(s), plan 881efe11b879

  dist[flavour=dev,target=lune]    dist/lune/dev.luau    17 rules
  dist[flavour=dev,target=roblox]  dist/roblox/dev.luau  17 rules
  dist[flavour=min,target=lune]    dist/lune/min.luau    17 rules
  dist[flavour=min,target=roblox]  dist/roblox/min.luau  17 rules
```

```sh
pcmp build dist                    # every combination
pcmp build 'dist[target=roblox]'   # one, by exact identifier
pcmp build '*target=roblox*'       # across an axis
```

Each axis is also a var, so `PCMP_TARGET` folds away in builds that do not need it.

## The `pcmp` API

Luau manifests only, and the manifest's only channel outward.

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
```

Both read `--env KEY=VALUE` first, then the process environment. `os`, `io`, `require`,
`loadstring`, `getfenv`, `setfenv`, `collectgarbage` and `math.random` are unavailable,
so two runs of the same manifest cannot disagree.

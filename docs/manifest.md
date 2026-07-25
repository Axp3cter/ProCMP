---
description: Every field, in one annotated manifest.
---

# Manifest

Discovery starts in the working directory and walks up, trying each name in turn:

```
pcmp.json5   pcmp.json   pcmp.jsonc   pcmp.toml   pcmp.luau
```

Relative paths always resolve against the manifest's own directory, not the working
directory.

## One plan, whichever format

These three resolve to byte-identical plans, digest and all:

{% tabs %}
{% tab title="pcmp.json5" %}
```json5
{
  $schema: "./pcmp.schema.json",

  vars: { name: "app", version: "v0.0.0-dev" },

  profiles: {
    release: {
      entry: "src/init.luau",
      output: "dist/{name}.luau",
      define: { DEBUG: false },
      darklua: {
        generator: "dense",
        rules: ["compute_expression", "remove_unused_if_branch"],
      },
    },
  },
}
```
{% endtab %}

{% tab title="pcmp.toml" %}
```toml
[vars]
name    = "app"
version = "v0.0.0-dev"

[profiles.release]
entry  = "src/init.luau"
output = "dist/{name}.luau"
define = { DEBUG = false }

[profiles.release.darklua]
generator = "dense"
rules     = ["compute_expression", "remove_unused_if_branch"]
```
{% endtab %}

{% tab title="pcmp.luau" %}
```lua
return {
	vars = {
		name    = "app",
		version = pcmp.envOr("VERSION", "v0.0.0-dev"),
	},

	profiles = {
		release = {
			entry   = "src/init.luau",
			output  = "dist/{name}.luau",
			define  = { DEBUG = false },
			darklua = {
				generator = "dense",
				rules     = { "compute_expression", "remove_unused_if_branch" },
			},
		},
	},
}
```
{% endtab %}
{% endtabs %}

```
$ pcmp -m pcmp.json5 plan
1 task(s), plan c17e90d8cdc0

$ pcmp -m pcmp.toml plan
1 task(s), plan c17e90d8cdc0

$ pcmp -m pcmp.luau plan
1 task(s), plan c17e90d8cdc0
```

Luau is the only format that can compute a value rather than be given one, through
[`pcmp.env`](#the-pcmp-api). Every map is sorted before resolution, so the hash order of
a Luau table cannot reach the plan.

## Profile fields

Ten of ProCMP's own, plus [`darklua`](darklua.md), which is where everything that
configures a transformation lives.

| Field | |
| --- | --- |
| `abstract` | Never built. Exempt from `entry` and `output` |
| `extends` | Name of a profile to inherit from |
| `entry` | A file, or a directory processed as a tree |
| `output` | Destination, with `{token}` expansion |
| `sources` | Extra directories whose contents count as build inputs |
| `ignore` | Globs excluded from that input set |
| `vars` | Named strings. Each becomes a `{token}` and a `PCMP_<NAME>` constant |
| `define` | Compile-time constants substituted into your source |
| `header` | Lines written above each artifact after darklua runs |
| `loaders` | Ordered `pattern` to `use` pairs |
| `darklua` | darklua's own configuration, verbatim |

{% code title="pcmp.json5" %}
```json5
{
  vars: { name: "app", version: "v0.0.0-dev" },

  profiles: {
    base: {
      abstract: true,
      entry: "src/init.luau",
      output: "dist/{profile}/{name}.luau",
      sources: ["../shared"],
      ignore: ["**/Packages/**"],

      darklua: {
        bundle: { require_mode: "luau" },
      },
    },

    release: {
      extends: "base",
      define: { DEBUG: false, RETRIES: 3 },
      header: ["--!native", "-- {name} {version}"],

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

Every var is also a define, uppercased and prefixed. The reverse does not hold, because
a define can be a boolean or a number.

```lua
if DEBUG then
	print("verbose telemetry")
end

local channel: string = PCMP_CHANNEL
```

A define is a boolean, a finite number, or a string. The type reaches the cache key, so
`true` and `"true"` are different builds.

{% hint style="warning" %}
`_G.DEBUG` and `_G["DEBUG"]` work identically. `getgenv().DEBUG` does not: it is a
function call, so there is nothing to replace at build time.
{% endhint %}

## Values from outside

Three ways in, in order of precedence:

```sh
pcmp build --var version=v1.2.3 -D CHANNEL=beta
```

```lua
vars = { version = pcmp.env("VERSION") },                    -- errors when unset
vars = { version = pcmp.envOr("VERSION", "v0.0.0-dev") },    -- explicit fallback
```

`--var` and `--define` work whatever the format, and beat the manifest.

{% hint style="info" %}
There is no build-timestamp define, because it would break `pcmp verify`. Pass one in
with `--var` if you want it.
{% endhint %}

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

{% tabs %}
{% tab title="File to file" %}
```json5
entry: "src/init.luau", output: "dist/app.luau",
```

Bundled into one file if `darklua.bundle` is set.
{% endtab %}

{% tab title="Directory to directory" %}
```json5
entry: "src", output: "build",
```

Every file processed, structure preserved, no bundling.
{% endtab %}
{% endtabs %}

`header` applies to every `.luau` and `.lua` artifact either way.

`output` expands `{profile}`, every var, and every matrix axis. `{{` and `}}` are literal
braces. An unknown token is [`bad-template`](diagnostics.md), not an empty string.

## Loaders

Teaches darklua what to do with files it would otherwise ignore:

```json5
loaders: [
  { pattern: "**/*.png", use: "buffer/base64" },
  { pattern: "**/*.md", use: "string" },
  { pattern: "**/*.json", use: "json" },
]
```

Require them **with the extension**:

```lua
local config = require("@self/assets/config.json")
```

| `use` | |
| --- | --- |
| `copy` | Passed through untouched |
| `skip` | Excluded from the output |
| `luau` | Parsed and processed as source |
| `json`, `json_lines`, `toml`, `yaml` | Returned as parsed data |
| `string`, `buffer`, `bytes` | Returned as content |

`string`, `buffer` and `bytes` also take an encoding: `/base64`, `/zstd`, `/gzip` or
`/zlib`.

{% hint style="info" %}
An ordered list rather than darklua's map, because darklua takes the first match and a
Luau table iterates in hash order. Declaring loaders in both places is
[`darklua-loaders`](diagnostics.md).
{% endhint %}

## Matrix

One task per axis combination, named by its coordinates.

```
$ pcmp plan
4 task(s), plan 9b35cd36a0d9

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

Each axis is also a var, so a matrix over `target` gives `{target}` and `PCMP_TARGET`.

## The `pcmp` API

Luau manifests only, and the manifest's only channel outward.

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
```

Both read `--env KEY=VALUE` first, then the process environment.

{% hint style="info" %}
`os`, `io`, `require`, `loadstring`, `getfenv`, `setfenv`, `collectgarbage` and
`math.random` are unavailable, so two evaluations of one manifest cannot disagree.
`print` writes to stderr, leaving stdout clean for `--json`.
{% endhint %}

{% content-ref url="darklua.md" %}
[darklua.md](darklua.md)
{% endcontent-ref %}

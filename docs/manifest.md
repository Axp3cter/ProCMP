---
description: Every field, and what it does.
icon: file-code
---

# Manifest

Discovery starts in the working directory and walks up, trying each name in turn.

```
pcmp.json5   pcmp.json   pcmp.jsonc   pcmp.toml   pcmp.luau
```

Format comes from the extension, never from the content. Relative paths always resolve against the manifest's own directory, not the working directory.

{% hint style="success" %}
All five formats resolve to the same plan, digest included.

The digest does not depend on where the project is checked out, or on how the manifest is arranged, only on what it means.
{% endhint %}

{% tabs %}
{% tab title="pcmp.json5" %}
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
{% endtab %}

{% tab title="pcmp.toml" %}
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
{% endtab %}

{% tab title="pcmp.luau" %}
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

Only this format can compute a value rather than be given one, through [the pcmp API](manifest.md#the-pcmp-api).
{% endtab %}
{% endtabs %}

## Top level

| | |
| --- | --- |
| `vars` | named values every profile starts from |
| `templates` | never built, and exist to be extended |
| `profiles` | built, once each, or once per axis combination |

A name may appear in `templates` or in `profiles`, not both. `extends` looks in one namespace so that it needs no precedence rule, and a name in both is `name-collision`.

## Profile fields

| | |
| --- | --- |
| `extends` | a template or profile to inherit from |
| `entry` | a file to bundle, or a directory to process as a tree |
| `output` | where it goes |
| `sources` | extra files and directories that count as build inputs |
| `ignore` | globs excluded from that input set |
| `vars` | named values, each becoming a `{token}` and a `PCMP_<NAME>` constant |
| `define` | constants substituted into your source |
| `header` | lines written above each artifact, after darklua discards comments |
| `loaders` | ordered `pattern` to `use` pairs |
| `darklua` | [darklua's own configuration](darklua.md), verbatim |
| `axes` | expands the profile into one task per combination |

`entry`, `output` and `header` are templates. `{profile}`, every var and every axis expand, `{{` and `}}` are literal braces, and an unknown token is an error rather than an empty string.

## Vars and defines

A var names a value used to build a path, a header, or a constant. A define names a value substituted into your source.

Every var is also a define, uppercased and prefixed. The reverse does not hold, because a define needs no name that is also a path token.

```json5
vars:   { name: "app", retries: 3 },
define: { DEBUG: false }
```

Both take a string, a number or a boolean. The type survives into the emitted Luau, so `retries: 3` is a number your source can do arithmetic on, and it reaches the cache key type-tagged, which is why `true` and `"true"` are different builds.

{% hint style="warning" %}
An integer must survive a round trip through a double, which bounds it at 2^53.

Past that is [`bad-define`](diagnostics.md#bad-define), not a silent change of value.
{% endhint %}

## Entry and output

{% tabs %}
{% tab title="File to file" %}
```json5
entry: "src/init.luau", output: "dist/app.luau",
```

Bundled into one file when `darklua.bundle` is set.
{% endtab %}

{% tab title="Directory to directory" %}
```json5
entry: "src", output: "build",
```

Every file processed, structure preserved, no bundling.
{% endtab %}
{% endtabs %}

`header` applies to whatever darklua emits as source, meaning `.luau` and `.lua`, or whatever `darklua.lua_extension` says.

Two tasks writing one path is [`output-collision`](diagnostics.md#output-collision). A task writing inside an entry tree is [`output-in-inputs`](diagnostics.md#output-in-inputs).

{% hint style="danger" %}
An `output` that climbs out of the project with `..` is legal, and occasionally meant.

`pcmp check` reports it as [`output-outside-root`](diagnostics.md#output-outside-root), because it means ProCMP is writing where nobody reading the manifest expects it to.
{% endhint %}

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

| `use` | |
| --- | --- |
| `copy` | passed through untouched |
| `skip` | excluded from the output |
| `luau` | parsed and processed as source |
| `json`, `json_lines`, `toml`, `yaml` | returned as parsed data |
| `string`, `buffer`, `bytes` | returned as content |

The content forms also take an encoding, one of `/base64`, `/zstd`, `/gzip` or `/zlib`.

{% hint style="info" %}
darklua takes the first pattern that matches, so order decides.

A list always has an order. A map has one only in a data format, so the map spelling is refused from a Luau manifest, where a table does not.
{% endhint %}

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

Luau manifests only, and the manifest's only channel outward. Every entry records what it answered with, which is what [Determinism](determinism.md) is about.

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
pcmp.now()                        -- RFC 3339 UTC
pcmp.epoch()                      -- seconds, consistent with now() in one run
pcmp.read("VERSION")              -- a file, relative to the manifest
pcmp.root                         -- the manifest's directory
pcmp.darklua                      -- the linked darklua version
```

{% hint style="warning" %}
There is deliberately no `pcmp.exec`.

Recording a subprocess honestly would mean hashing its whole environment, and a ledger that lies is worse than no ledger. Pass a git SHA in with `--var`.
{% endhint %}

The sandbox removes `os`, `io`, `load`, `require`, `debug`, `coroutine` and `math.random`, and bounds evaluation by steps and by memory.

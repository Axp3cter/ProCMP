---
description: Every field, in one annotated manifest.
---

# Manifest

`pcmp.luau`, `pcmp.json`, `pcmp.jsonc`, `pcmp.json5` or `pcmp.toml` — all resolve to the
same plan. JSON is parsed leniently, so comments and trailing commas are fine. Unknown
keys are rejected.

Discovery starts in the working directory and walks up, so `pcmp` works from anywhere
inside a project. Relative paths always resolve against the manifest's own directory,
never against where the command was run.

## The split

ProCMP owns eight profile fields. Everything that configures a transformation lives
under `darklua` and is darklua's own configuration format, deserialised by darklua.

<table><thead><tr><th width="230">ProCMP</th><th></th></tr></thead><tbody>
<tr><td><code>abstract</code>, <code>extends</code></td><td>Inheritance.</td></tr>
<tr><td><code>entry</code>, <code>output</code></td><td>What is read and where it is written.</td></tr>
<tr><td><code>sources</code>, <code>ignore</code></td><td>What counts as a build input. See <a href="inputs.md">Inputs</a>.</td></tr>
<tr><td><code>vars</code>, <code>define</code></td><td>Tokens and compile-time constants.</td></tr>
<tr><td><code>header</code></td><td>Lines written after darklua runs.</td></tr>
<tr><td><code>loaders</code></td><td>The one darklua setting ProCMP re-spells, because order matters.</td></tr>
<tr><td><code>darklua</code></td><td>Everything else, verbatim.</td></tr>
</tbody></table>

## Everything at once

{% code title="pcmp.luau" %}
```lua
return {
	-- Each becomes a {token} and a PCMP_<NAME> constant.
	vars = {
		name    = "app",
		version = pcmp.envOr("VERSION", "v0.0.0-dev"),
	},

	profiles = {
		base = {
			abstract = true,                          -- never built; exempt from entry/output
			entry    = "src/init.luau",               -- a file, or a directory
			output   = "dist/{profile}/{name}.luau",
			sources  = { "../shared" },               -- extra input roots
			ignore   = { "**/Packages/**" },          -- excluded from the input set

			darklua = {
				bundle = {
					require_mode = { name = "luau", aliases = { pkg = "./Packages" } },
					excludes     = { "@lune/**" },
				},
			},
		},

		release = {
			extends = "base",
			define  = { DEBUG = false, RETRIES = 3, CHANNEL = "stable" },

			-- Written above each artifact after darklua runs, so it survives minification.
			header = {
				"--!native",
				"--!optimize 2",
				"-- {name} {version} — generated, do not edit",
			},

			-- Ordered: darklua takes the first pattern that matches.
			loaders = {
				{ pattern = "**/*.model.json", use = "copy" },
				{ pattern = "**/*.json",       use = "json" },
				{ pattern = "**/*.md",         use = "string" },
			},

			darklua = {
				generator      = { name = "dense", column_span = 120 },
				apply_to_files = { "src/**" },
				skip_files     = { "**/*.test.luau" },
				rules = {
					"compute_expression",
					"remove_unused_if_branch",
					"rename_variables",
				},
			},
		},
	},

	matrix = {
		dist = {
			base   = "base",
			axes   = { target = { "roblox", "lune" } },
			output = "dist/{target}/{name}.luau",
			define = { BUNDLED = true },
		},
	},
}
```
{% endcode %}

Every key under `darklua` is documented by
[darklua's configuration reference](https://darklua.com/docs/config/), and a key the
linked version does not know is rejected with darklua's own message and the JSON that
was emitted.

## Vars

Named strings. Each is a `{token}` in `output` and `header`, and a `PCMP_<NAME>`
constant in the source.

```lua
vars = { name = "app", channel = "stable" },
output = "dist/{channel}/{name}.luau",
```

```lua
if PCMP_CHANNEL == "stable" then ... end
```

`{profile}` is always available. Matrix axes contribute one token each. `--var` beats
all of them:

```sh
pcmp build --var version="$(git describe --tags)"
```

There is no `project` or `version` field. Both were `vars` entries with a special case
attached, and the special case bought nothing a token could not do.

## Inheritance

Nearer wins, field by field. `vars` and `define` accumulate; `darklua` merges key by
key, so a profile can set `generator` without restating the `bundle` it inherited.

```lua
base    = { abstract = true, define = { A = 1, B = 1 }, header = { "--!native" } },
release = { extends = "base", define = { B = 2 } },
-- release resolves to define { A = 1, B = 2 } and header { "--!native" }
```

Every list replaces outright, so a child clears one it inherited by declaring it empty:

```lua
plain = { extends = "base", header = {} },
```

A cycle is reported as `cyclic-extends` with the full chain.

## Entry and output

A file in, a file out — bundled, if `darklua.bundle` is set:

```lua
entry = "src/init.luau", output = "dist/app.luau",
```

A directory in, a directory out — every file processed, structure preserved, no
bundling:

```lua
entry = "src", output = "build",
```

`header` applies to every artifact either way, and `pcmp verify` compares the whole tree.

## Output tokens

```lua
output = "dist/{profile}/{name}-{version}.luau"
```

`{profile}`, every var, and every matrix axis. `{{` and `}}` are literal braces. An
unknown token is `bad-template`, not an empty string.

## Loaders

`loaders` teaches darklua what to do with files it would otherwise ignore, which is how
you embed assets:

```lua
loaders = {
	{ pattern = "**/*.png",  use = "buffer/base64" },   -- embedded as a buffer
	{ pattern = "**/*.md",   use = "string" },          -- embedded as a string
	{ pattern = "**/*.json", use = "json" },            -- parsed into a table
}
```

Require them **with the extension** — that is how darklua tells a data file from a
module:

```lua
local config = require("@self/assets/config.json")
local readme = require("@self/assets/readme.md")
```

Available: `copy`, `skip`, `json`, `json_lines`, `toml`, `yaml`, `string`, `buffer`,
`bytes`, and the encoded forms `string/base64`, `buffer/zstd`, `bytes/gzip` and so on.

{% hint style="info" %}
This is the one darklua setting ProCMP re-spells. darklua takes the first pattern that
matches, and a Luau table iterates in hash order — a map here would let the manifest
format decide which pattern wins. Declaring loaders in both `loaders` and
`darklua.loaders` is `darklua-loaders`, an error.
{% endhint %}

## Matrix

One task per combination, named by its coordinates.

```
$ pcmp plan
4 task(s) — plan 881efe11b879

  dist[flavour=dev,target=lune]    out/lune/dev.luau    17 rules
  dist[flavour=dev,target=roblox]  out/roblox/dev.luau  17 rules
  dist[flavour=min,target=lune]    out/lune/min.luau    17 rules
  dist[flavour=min,target=roblox]  out/roblox/min.luau  17 rules
```

```sh
pcmp build dist                    # every combination
pcmp build 'dist[target=roblox]'   # one, by exact identifier
pcmp build '*target=roblox*'       # across an axis
```

Each axis is also a var, so `PCMP_TARGET` folds away in the builds that do not need it.

## The `pcmp` API

Luau manifests only, and the manifest's only channel outward.

```lua
pcmp.env("VERSION")               -- errors when unset
pcmp.envOr("VERSION", "v0.0.0")   -- explicit fallback
```

Both read `--env KEY=VALUE` first, then the process environment.

`os`, `io`, `require`, `loadstring`, `getfenv`, `setfenv`, `collectgarbage` and
`math.random` are unavailable, so two runs of the same manifest cannot disagree.

## Other formats

{% tabs %}
{% tab title="JSON" %}
```json
{
  "$schema": "./pcmp.schema.json",
  "vars": { "name": "app", "version": "v1.0.0" },
  "profiles": {
    "release": {
      "entry": "src/init.luau",
      "output": "dist/{name}.luau",
      "define": { "DEBUG": false },
      "darklua": {
        "generator": "dense",
        "rules": ["compute_expression", "rename_variables"]
      }
    }
  }
}
```
{% endtab %}

{% tab title="TOML" %}
```toml
[vars]
name = "app"
version = "v1.0.0"

[profiles.release]
entry = "src/init.luau"
output = "dist/{name}.luau"

[profiles.release.define]
DEBUG = false

[profiles.release.darklua]
generator = "dense"
rules = ["compute_expression", "rename_variables"]
```
{% endtab %}
{% endtabs %}

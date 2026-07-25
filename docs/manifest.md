---
description: Every field, in one annotated manifest.
---

# Manifest

`pcmp.json5`, `pcmp.json`, `pcmp.jsonc`, `pcmp.toml` and `pcmp.luau` all resolve to the
same plan, and that is discovery order. JSON is parsed leniently, so comments and
trailing commas are fine. Unknown keys are rejected.

`pcmp init` writes JSON5, because it is plain data any tool can read or rewrite and
`$schema` gives an editor validation with no further setup. Luau is the one format that
can compute a value rather than be given one.

Discovery starts in the working directory and walks up, so `pcmp` works from anywhere
inside a project. Relative paths always resolve against the manifest's own directory,
never against where the command was run.

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
			abstract = true,                          -- never built, exempt from entry/output
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
				"-- {name} {version}, generated, do not edit",
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
if PCMP_CHANNEL == "stable" then
	enableTelemetry()
end
```

`{profile}` is always available. Matrix axes contribute one token each. Two names that
differ only in case are `bad-var`, because both would become one `PCMP_<NAME>`. `--var`
beats all of them:

```sh
pcmp build --var version="$(git describe --tags)"
```

## Inheritance

Nearer wins, field by field. `vars` and `define` accumulate. `darklua` merges key by
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

A file in, a file out, bundled if `darklua.bundle` is set:

```lua
entry = "src/init.luau", output = "dist/app.luau",
```

A directory in, a directory out. Every file processed, structure preserved, no
bundling:

```lua
entry = "src", output = "build",
```

`header` applies to every artifact either way, and `pcmp verify` compares the whole tree.

`output` expands `{profile}`, every var, and every matrix axis. `{{` and `}}` are
literal braces. An unknown token is `bad-template`, not an empty string.

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

Require them **with the extension**, which is how darklua tells a data file from a
module:

```lua
local config = require("@self/assets/config.json")
local readme = require("@self/assets/readme.md")
```

Available: `copy`, `skip`, `json`, `json_lines`, `toml`, `yaml`, `string`, `buffer`,
`bytes`, and encoded forms such as `string/base64` and `buffer/zstd`.

An ordered list rather than darklua's map, because darklua takes the first pattern that
matches and a Luau table iterates in hash order. Declaring loaders in both `loaders` and
`darklua.loaders` is `darklua-loaders`, an error.

## Matrix

One task per combination, named by its coordinates.

```
$ pcmp plan
4 task(s), plan 881efe11b879

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


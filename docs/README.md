---
description: Bundle one Luau source tree into many build targets from a single manifest.
---

# ProCMP

One source tree, many artifacts: a minified release, a readable debug build, a Roblox
variant, a Lune variant. All described in one manifest and built in one command.

[darklua](https://darklua.com) is linked in, so there is no second binary to install and
nothing to keep in sync.

{% code title="pcmp.luau" %}
```lua
return {
	vars = {
		name    = "app",
		version = pcmp.envOr("VERSION", "v0.0.0-dev"),
	},

	profiles = {
		base = {
			abstract = true,
			entry    = "src/init.luau",
			output   = "dist/{profile}/{name}.luau",
			darklua  = {
				bundle = { require_mode = "luau" },
			},
		},

		debug = {
			extends = "base",
			define  = { DEBUG = true },
			darklua = {
				generator = "readable",
				rules     = { "compute_expression" },
			},
		},

		release = {
			extends = "base",
			define  = { DEBUG = false },
			darklua = {
				generator = "dense",
				rules     = {
					"compute_expression",
					"remove_unused_if_branch",
					"remove_types",
					"remove_comments",
					"rename_variables",
				},
			},
		},
	},
}
```
{% endcode %}

```
$ pcmp build
  built   debug    dist/debug/app.luau  (1 ms)
  built   release  dist/release/app.luau  (0 ms)

2 built, 0 cached, 0 failed
```

## What it gives you

<table data-view="cards">
<thead><tr><th></th><th></th></tr></thead>
<tbody>
<tr>
<td><strong>Compile-time constants</strong></td>
<td><code>define</code> values become literals, and the branches they disable are deleted from the artifact rather than shipped and skipped.</td>
</tr>
<tr>
<td><strong>Profiles and matrices</strong></td>
<td>Share settings with <code>extends</code>, or expand a matrix into one task per target.</td>
</tr>
<tr>
<td><strong>Five manifest formats</strong></td>
<td>Luau, JSON, JSONC, JSON5 and TOML all resolve to the same plan.</td>
</tr>
<tr>
<td><strong>Reproducible by construction</strong></td>
<td><code>pcmp verify</code> builds twice and byte-compares, so nondeterminism fails CI instead of reaching a release.</td>
</tr>
</tbody>
</table>

{% content-ref url="install.md" %}
[install.md](install.md)
{% endcontent-ref %}

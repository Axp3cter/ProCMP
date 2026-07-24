<h1 align="center">ProCMP</h1>

<p align="center">Bundle one Luau source tree into many build targets from a single manifest.</p>

<p align="center">
  <a href="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="https://github.com/Proton-Utilities/ProCMP/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Proton-Utilities/ProCMP?include_prereleases"></a>
</p>

---

One source tree, many artifacts. A minified release, a readable debug build, a Roblox
variant, a Lune variant, all from one manifest.

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is a single static
binary with nothing to install alongside it and no version to keep in sync.

## Install

```sh
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

Or download a binary from [releases](https://github.com/Proton-Utilities/ProCMP/releases).

## Manifest

```luau
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
			darklua  = { bundle = { require_mode = "luau" } },
		},

		debug = {
			extends = "base",
			define  = { DEBUG = true },
			darklua = { generator = "readable", rules = { "compute_expression" } },
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

```sh
pcmp plan     # what would be built
pcmp check    # lint the manifest and the plan
pcmp build    # build it
pcmp watch    # rebuild on every change
pcmp verify   # prove the output is reproducible
```

JSON, JSONC, JSON5 and TOML manifests resolve to the same plan.

## Defines are removed, not disabled

```luau
if _G.DEBUG then
	print("verbose telemetry")
end
```

In a release build that is not `if false then`. The value is injected as an AST node, so
the condition folds and the branch is **gone from the artifact**.

## Design

**The `darklua` block is darklua's config, verbatim.** No rule names of our own, no
presets, no reordering. Nothing the linked darklua supports is out of reach, and
`pcmp explain` prints the exact `.darklua.json` a task compiles to.

**Inputs are derived, not guessed.** The hashed set comes from the plan: every file
under your roots, minus your outputs and the cache. No extension allowlist, so a `.json`
asset behind a content loader invalidates the build that reads it.

**Findings accumulate.** Four mistakes report four, each with a stable code and a fix.

**Reproducible by construction.** `pcmp verify` builds twice and byte-compares, so
nondeterminism fails CI instead of reaching a release.

## Documentation

[Installation, concepts, manifest reference, CLI reference, and the full diagnostic
catalogue](https://github.com/Proton-Utilities/ProCMP/tree/main/docs).

## License

MIT, see [LICENSE](LICENSE).

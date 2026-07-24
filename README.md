<h1 align="center">ProCMP</h1>

<p align="center">Bundle one Luau source tree into many build targets from a single manifest.</p>

<p align="center">
  <a href="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="https://github.com/Proton-Utilities/ProCMP/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Proton-Utilities/ProCMP?include_prereleases"></a>
</p>

---

ProCMP turns one Luau source tree into many artifacts — a minified release, a readable
debug build, a Roblox variant, a Lune variant — from a single manifest. It links
[darklua](https://darklua.com) in as a library, so there is no `darklua` binary to
install, no `PATH` to get wrong, and no version drift between what you pinned and what
is on the machine.

The same inputs always produce the same bytes. `pcmp verify` proves it.

## Install

Download a binary from [releases](https://github.com/Proton-Utilities/ProCMP/releases),
or build from source:

```sh
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

## Use

A manifest, `pcmp.luau`:

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

Then:

```sh
pcmp plan     # what would be built
pcmp check    # lint the manifest and the plan
pcmp build    # build it
pcmp verify   # prove the output is reproducible
```

`define` entries become real constants. In a release build, this:

```luau
if _G.DEBUG then
	print("verbose telemetry")
end
```

is not merely false — it is **gone from the artifact**, along with the branch, because
the value is injected as an AST node and then folded away.

## Why

**darklua is linked in**, not shelled out to, so `pcmp` is one static binary with no
external version to drift.

**The `darklua` block is darklua's config, verbatim.** ProCMP adds no rule names, no
presets and no reordering — nothing the linked darklua supports is unreachable, and
`pcmp explain` prints the exact `.darklua.json` a task compiles down to.

**Manifests are Luau, JSON, JSONC, JSON5 or TOML**, all resolving to the same plan. The
Luau front end is a real interpreter with every entropy source revoked, so config can
compute without going nondeterministic.

**Inputs are derived, not guessed.** The hashed set comes from the plan — every file
under your roots, minus your outputs and the cache. No extension allowlist, so a `.json`
asset behind a content loader invalidates the build it belongs to.

**Findings accumulate.** A manifest with four mistakes reports four, each with a stable
code and a fix.

## Documentation

**[procmp.dev](https://github.com/Proton-Utilities/ProCMP/tree/main/docs)** — installation,
concepts, manifest reference, CLI reference, and the full diagnostic catalogue.

## License

MIT — see [LICENSE](LICENSE).

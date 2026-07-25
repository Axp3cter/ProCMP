<h1 align="center">ProCMP</h1>

<p align="center">Bundle one Luau source tree into many build targets from a single manifest.</p>

<p align="center">
  <a href="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="https://github.com/Proton-Utilities/ProCMP/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Proton-Utilities/ProCMP?include_prereleases"></a>
</p>

---

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is one static binary
with nothing to install alongside it.

## Install

```sh
rokit add Proton-Utilities/ProCMP pcmp
aftman add Proton-Utilities/ProCMP pcmp
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

## Use

```json5
// pcmp.json5
{
  $schema: "./pcmp.schema.json",

  vars: {
    name: "app",
    version: "v0.0.0-dev",
  },

  profiles: {
    release: {
      entry: "src/init.luau",
      output: "dist/{name}.luau",
      define: { DEBUG: false },
      darklua: {
        generator: "dense",
        bundle: { require_mode: "luau" },
        rules: ["compute_expression", "remove_unused_if_branch"],
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

`pcmp init` writes that file and its schema. Luau, JSON, JSONC and TOML manifests
resolve to the same plan, and `pcmp init --format luau` scaffolds the Luau one.

`define` values are injected as AST nodes, so `if _G.DEBUG then` folds away and the
branch is gone from the artifact.

The `darklua` block is darklua's own configuration, deserialised by darklua. `pcmp
explain` prints what a task compiles to, which is a valid `.darklua.json`.

## Documentation

[docs/](https://github.com/Proton-Utilities/ProCMP/tree/main/docs)

## License

MIT, see [LICENSE](LICENSE).

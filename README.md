<h1 align="center">ProCMP</h1>

<p align="center">Bundle one Luau source tree into many build targets from a single manifest.</p>

<p align="center">
  <a href="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="https://github.com/Proton-Utilities/ProCMP/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Proton-Utilities/ProCMP?include_prereleases"></a>
</p>

---

Links in [darklua](https://darklua.com) as a library.

## Install

```sh
rokit add Proton-Utilities/ProCMP pcmp
aftman add Proton-Utilities/ProCMP pcmp
cargo install --locked --git https://github.com/Proton-Utilities/ProCMP
```

## Use

```sh
pcmp init
```

```json5
// pcmp.json5
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

JSON, JSONC, TOML and Luau manifests resolve to the same plan.

## Documentation

[docs/](https://github.com/Proton-Utilities/ProCMP/tree/main/docs)

## License

MIT, see [LICENSE](LICENSE).

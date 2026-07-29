<h1 align="center">ProCMP</h1>

<p align="center">Bundle one Luau source tree into many build targets from a single manifest.</p>

<p align="center">
  <a href="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Proton-Utilities/ProCMP/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="https://github.com/Proton-Utilities/ProCMP/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Proton-Utilities/ProCMP?include_prereleases"></a>
</p>

---

A minified release, a readable debug build, a Roblox variant and a Lune variant, described
in one file and built by one command. [darklua](https://darklua.com) is linked in as a
library, so `pcmp` is a single binary with nothing to install alongside it.

```sh
rokit add Proton-Utilities/ProCMP
pcmp init
pcmp build
```

## Reproducible, including the parts that change

A build is a function of what the manifest declares. Everything it takes from outside —
an environment variable, a file, the clock — is recorded, so a version stamp or a build
timestamp does not cost you reproducibility:

```sh
pcmp build --lock      # build, and write down what it read
pcmp build --frozen    # build again from that record, and prove it matches
```

`--frozen` reproduces a build exactly, timestamp included, because the timestamp is one of
the inputs the lock pins. `pcmp check` says so when a manifest reads something and no lock
records it.

## Commands

`build` · `plan` · `check` · `watch` · `init` · `schema` · `explain`

`pcmp help` describes every flag, and `pcmp explain <CODE>` describes any diagnostic you
see. Neither is repeated here, so neither can go stale.

## Documentation

[docs/](docs/) — a tutorial, the manifest reference, and how determinism works.

## License

MIT, see [LICENSE](LICENSE).

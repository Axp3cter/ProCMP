---
description: Getting the binary.
---

# Install

```sh
rokit add Proton-Utilities/ProCMP
aftman add Proton-Utilities/ProCMP
cargo install --locked --git https://github.com/Proton-Utilities/ProCMP
```

The binary is called `pcmp`. darklua is linked in as a library, so there is nothing else
to install and no version to keep in step — `pcmp --version` prints the darklua it was
built against.

## Editor completion

```sh
pcmp schema > pcmp.schema.json          # JSON Schema, for a data manifest
pcmp schema --format luau > pcmp.d.luau # type definitions, for a Luau manifest
```

Point at the schema from the manifest itself:

```json5
{ $schema: "./pcmp.schema.json" }
```

Neither file is written by `pcmp init`, because a generated file committed to a repository
goes stale the next time you upgrade and nothing tells you. If you do commit one,
`pcmp check` reports `stale-schema` when it stops matching.

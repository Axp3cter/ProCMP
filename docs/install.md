---
title: Install
description: Getting the binary, and teaching your editor about the manifest.
icon: lucide/download
---

# Install

=== "rokit"

    ```sh
    rokit add Proton-Utilities/ProCMP
    ```

=== "aftman"

    ```sh
    aftman add Proton-Utilities/ProCMP
    ```

=== "cargo"

    ```sh
    cargo install --locked --git https://github.com/Proton-Utilities/ProCMP
    ```

The binary is called `pcmp`. darklua is linked in, so there is no second tool to install and no version to keep in step.

```console
$ pcmp --version
4.0.0
darklua 0.19.0
```

The darklua line is not decoration. It is part of every cache key, so an upgrade that changes emitted bytes rebuilds rather than serving a stale artifact.

## Editor completion

Neither file below is written by `pcmp init`. A generated file committed to a repository goes stale on the next upgrade with nothing to notice, so generating one is a choice you make rather than a default you inherit.

=== "Data manifest"

    ```sh
    pcmp schema > pcmp.schema.json
    ```

    Point at it from the manifest itself, and any JSON-aware editor validates as you type.

    ```json5 title="pcmp.json5"
    { $schema: "./pcmp.schema.json" }
    ```

=== "Luau manifest"

    ```sh
    pcmp schema --format luau > pcmp.d.luau
    ```

    The output is a definition file, which is why it ends in `declare pcmp: Api`. Name it in your `luau-lsp` settings and completion covers both the manifest's own shape and the `pcmp` globals.

    ```json title=".vscode/settings.json"
    {
      "luau-lsp.types.definitionFiles": ["pcmp.d.luau"]
    }
    ```

!!! info "Both are generated from the parser"

    `pcmp schema` and `pcmp schema --format luau` are derived from the same Rust type the manifest is deserialised into, so neither can describe a field this binary does not accept.

    If you do commit one, [`stale-schema`](diagnostics.md#stale-schema) tells you when it stops matching.

## Where things live

`pcmp.lock`

:   Committed. Records what a build read from outside the manifest and what it produced, so a diff of it in review shows exactly what changed about a build.

`.pcmp/`

:   Ignored. The local build cache, and disposable. Deleting it costs one rebuild and nothing else.

```gitignore title=".gitignore"
.pcmp/
```

Both live beside the manifest. `--cache-dir` moves the second one, which is worth doing when a CI runner caches a directory of its own choosing.

```sh
pcmp build --cache-dir "$RUNNER_TEMP/pcmp"
```

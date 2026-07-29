---
title: Install
description: Getting the binary, and teaching your editor about the manifest.
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

The darklua line is part of every cache key, so an upgrade that changes emitted bytes rebuilds rather than serving a stale artifact.

## Ignore the cache

`pcmp` writes two things beside the manifest. `pcmp.lock` is committed and described under [Reproducing a build](cli.md#reproducing-a-build). `.pcmp/` is the local build cache, and deleting it costs one rebuild and nothing else.

```gitignore title=".gitignore"
.pcmp/
```

`--cache-dir` moves the cache, which is worth doing when a CI runner caches a directory of its own choosing.

```sh
pcmp build --cache-dir "$RUNNER_TEMP/pcmp"
```

## Editor completion

Both files below are generated from the same Rust type the manifest is deserialised into, so neither can describe a field this binary does not accept. Neither is written by `pcmp init`, because a generated file committed to a repository goes stale on the next upgrade with nothing to notice. If you do commit one, [`stale-schema`](diagnostics.md#stale-schema) tells you when it stops matching.

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

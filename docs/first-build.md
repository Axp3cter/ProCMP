---
title: Your first build
description: From an empty project to two artifacts and a cache that holds.
icon: lucide/rocket
---

# Your first build

1.  **Scaffold.**

    ```sh
    pcmp init
    ```

    Writes `pcmp.json5` beside your `src/`, and nothing else. Open it, because everything below is a walk through what it says.

2.  **Build.**

    ```console
    $ pcmp build --var version=v1.0.0
    plan  9b35cd36a0d9

      built   dev      dist/dev/app.luau
      built   release  dist/release/app.luau

    2 built, 0 cached, 0 failed
    ```

3.  **Build again.**

    ```console
    $ pcmp build
    0 built, 2 cached, 0 failed
    ```

    Nothing changed, so nothing ran. When something does run, ask why.

    ```console
    $ pcmp plan --why
    plan  9b35cd36a0d9

      stale   release  dist/release/app.luau  a source file changed
      fresh   dev      dist/dev/app.luau

    1 stale, 1 fresh, 0 failed
    ```

    `plan --why` says what a build would do and does none of it, which is why it reports `stale` and `fresh` rather than `built` and `cached`.

## vars

```json5
vars: { name: "app", version: "v0.0.0-dev", retries: 3 }
```

Each var becomes two things.

`{name}`

:   A token you can put in a path or a header.

`PCMP_NAME`

:   A constant your source can read.

```lua
local channel: string = PCMP_VERSION
local tries: number = PCMP_RETRIES
```

Vars are typed, so `retries: 3` reaches Luau as a number rather than as the string `"3"`, and darklua can fold arithmetic on it.

Override one per build with `--var version=v1.2.3`.

## templates and profiles

```json5
templates: { base: { entry: "src/init.luau", output: "dist/{profile}/{name}.luau" } },
profiles:  { dev: { extends: "base" }, release: { extends: "base" } }
```

A **profile** is built. A **template** is not, and exists to be extended. Both are the same shape, and `extends` finds a name in either, so a profile can extend a profile just as easily.

Nearer wins, field by field. `vars` and `define` accumulate, `darklua` merges key by key, and a list replaces outright.

```json5
plain: { extends: "release", header: [] }
```

That clears an inherited `header`, because declaring a list empty is different from not mentioning it.

## define

```json5
define: { DEBUG: false }
```

A define is substituted into your source as a value, before anything else runs.

```lua
if DEBUG then
	print("verbose telemetry")
end
```

In `dev` that folds to `if true` and stays. In `release` it folds to `if false`, and `remove_unused_if_branch` deletes the block, so the artifact never ships dead code wrapped in a constant.

!!! warning "What a define can reach"

    `_G.DEBUG` and `_G["DEBUG"]` work the same way.

    `getgenv().DEBUG` does not, because it is a function call and there is nothing to replace at build time.

Misspell one and `pcmp check` says so, through [`unreachable-define`](diagnostics.md#unreachable-define).

```console
$ pcmp check
warning[unreachable-define]: `DEBUGG` appears in no source `release` reads
  at:   profiles.release.define.DEBUGG
  help: nothing will be substituted, so check the spelling on both sides

0 errors, 1 warning
```

## axes

One profile, many builds.

```json5 title="pcmp.json5"
dist: {
  extends: "base",
  output: "dist/{target}/{flavour}.luau", // (1)!
  axes: {
    flavour: ["min", "dev"], // (2)!
    target: {
      roblox: { darklua: { bundle: { require_mode: "path" } } }, // (3)!
      lune:   { darklua: { bundle: { require_mode: "luau" } } },
    },
  },
}
```

1.  Every axis is also a var, so its name is available as a `{token}` here and as `PCMP_TARGET` in your source.
2.  The list form. Two axes of two values give four combinations.
3.  The map form, where a value carries an overlay that can set any profile field, not only a path.

```console
$ pcmp plan
plan  9b35cd36a0d9

  dist[flavour=dev,target=lune]    dist/lune/dev.luau    10 rules
  dist[flavour=dev,target=roblox]  dist/roblox/dev.luau  10 rules
  dist[flavour=min,target=lune]    dist/lune/min.luau    12 rules
  dist[flavour=min,target=roblox]  dist/roblox/min.luau  12 rules

4 tasks
```

Build all of them, one of them, or a slice.

```sh
pcmp build dist
pcmp build 'dist[flavour=min,target=roblox]'
pcmp build dist --axis target=roblox
```

[Manifest reference :octicons-arrow-right-24:](manifest.md){ .md-button .md-button--primary }

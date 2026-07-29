---
description: From an empty project to two artifacts and a cache that holds.
icon: rocket-launch
---

# Your first build

{% stepper %}
{% step %}
### Scaffold

```sh
pcmp init
```

Writes `pcmp.json5` beside your `src/`, and nothing else. Open it, because everything below is a walk through what it says.
{% endstep %}

{% step %}
### Build

```sh
pcmp build --var version=v1.0.0
```

```
plan  9b35cd36a0d9

  built   dev      dist/dev/app.luau      12 ms
  built   release  dist/release/app.luau  14 ms

2 built, 0 cached, 0 failed
```
{% endstep %}

{% step %}
### Build again

```
0 built, 2 cached, 0 failed
```

Nothing changed, so nothing ran. When something does run, ask why.

```sh
pcmp plan --why
```

```
  stale   release  dist/release/app.luau  a source file changed
  fresh   dev      dist/dev/app.luau
```

`plan --why` says what a build would do and does none of it, which is why it reports `stale` and `fresh` rather than `built` and `cached`.
{% endstep %}
{% endstepper %}

## vars

```json5
vars: { name: "app", version: "v0.0.0-dev", retries: 3 }
```

Each var becomes two things.

| | |
| --- | --- |
| `{name}` | a token you can put in a path or a header |
| `PCMP_NAME` | a constant your source can read |

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

{% hint style="warning" %}
`_G.DEBUG` and `_G["DEBUG"]` work the same way.

`getgenv().DEBUG` does not, because it is a function call and there is nothing to replace at build time.
{% endhint %}

Misspell one and `pcmp check` says so, through [`unreachable-define`](diagnostics.md#unreachable-define).

```
warning[unreachable-define]: `DEBUGG` appears in no source `release` reads
  at:   profiles.release.define.DEBUGG
```

## axes

One profile, many builds.

```json5
dist: {
  extends: "base",
  output: "dist/{target}/{flavour}.luau",
  axes: {
    flavour: ["min", "dev"],
    target: {
      roblox: { darklua: { bundle: { require_mode: "path" } } },
      lune:   { darklua: { bundle: { require_mode: "luau" } } },
    },
  },
}
```

An axis is a list of values, or a map from a value to settings of its own. The second form is why a target can change anything a profile can, not only a path.

```
$ pcmp plan
plan  9b35cd36a0d9

  dist[flavour=dev,target=lune]    dist/lune/dev.luau    10 rules
  dist[flavour=dev,target=roblox]  dist/roblox/dev.luau  10 rules
  dist[flavour=min,target=lune]    dist/lune/min.luau    12 rules
  dist[flavour=min,target=roblox]  dist/roblox/min.luau  12 rules

4 tasks
```

Each axis is also a var, so `{target}` and `PCMP_TARGET` both work.

Build all of them, one of them, or a slice.

```sh
pcmp build dist
pcmp build 'dist[flavour=min,target=roblox]'
pcmp build dist --axis target=roblox
```

{% content-ref url="manifest.md" %}
[manifest.md](manifest.md)
{% endcontent-ref %}

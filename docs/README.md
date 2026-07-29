---
description: One Luau source tree, many build targets, from a single manifest.
---

# ProCMP

Start with a project that has a `src/init.luau`:

```sh
pcmp init
```

That writes `pcmp.json5` and nothing else. Open it — everything below is a walk through
what it says.

## vars

```json5
vars: { name: "app", version: "v0.0.0-dev" }
```

A var is a named value. Each one becomes two things: a `{token}` you can use in a path,
and a `PCMP_<NAME>` constant your source can read.

```lua
local channel: string = PCMP_VERSION
```

Vars are typed. `retries: 3` gives you a number, not the string `"3"`, so darklua can fold
arithmetic on it.

Override one per build:

```sh
pcmp build --var version=v1.2.3
```

## templates and profiles

```json5
templates: { base: { entry: "src/init.luau", output: "dist/{profile}/{name}.luau" } },
profiles:  { dev: { extends: "base", … }, release: { extends: "base", … } }
```

A **profile** is a build. A **template** is not built; it exists to be extended. Both are
the same shape, and `extends` finds a name in either — so a profile can extend a profile
just as easily.

Nearer wins, field by field. `vars` and `define` accumulate, and `darklua` merges key by
key, so `release` can set `generator` without restating `bundle`. A list replaces
outright, so a child clears an inherited one by declaring it empty:

```json5
plain: { extends: "release", header: [] }
```

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

In `dev` that folds to `if true`, and stays. In `release` it folds to `if false`, and
`remove_unused_if_branch` deletes the whole block — so the release artifact does not ship
`if false then` around dead code.

If you misspell one, `pcmp check` tells you: `unreachable-define` fires when a define's
name appears in none of your sources.

## Building

```
$ pcmp build --var version=v1.0.0
  built   dev      dist/dev/app.luau
  built   release  dist/release/app.luau

2 built, 0 cached, 0 failed
```

Run it again and both say `cached`. Ask why not:

```
$ pcmp plan --why
  built  release  dist/release/app.luau  — a source file changed
```

## axes

One profile, many builds. Give it axes and it expands into one task per combination:

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

An axis is a list of values, or a map from a value to settings of its own — so a target
can change anything a profile can, not just a path. Each axis is also a var, so `{target}`
and `PCMP_TARGET` both work.

```
$ pcmp plan
4 task(s), plan 9b35cd36a0d9

  dist[flavour=dev,target=lune]    dist/lune/dev.luau
  dist[flavour=dev,target=roblox]  dist/roblox/dev.luau
  dist[flavour=min,target=lune]    dist/lune/min.luau
  dist[flavour=min,target=roblox]  dist/roblox/min.luau
```

Build one, or a slice:

```sh
pcmp build dist                                # every combination
pcmp build 'dist[flavour=min,target=roblox]'   # exactly one
pcmp build dist --axis target=roblox           # by coordinate
```

## What next

`pcmp plan <TASK>` prints everything a task resolved to, including the darklua
configuration it compiles to — which is a valid `.darklua.json` if you ever want to run
darklua directly.

[Determinism](determinism.md) is worth reading before you put a version stamp in a header.

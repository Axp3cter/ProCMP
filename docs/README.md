---
description: One Luau source tree, many build targets, from a single manifest.
---

# ProCMP

A minified release, a readable debug build, a Roblox variant and a Lune variant, all
described in one file and built by one command. [darklua](https://darklua.com) is linked
in as a library, so `pcmp` is a single binary.

## Quickstart

{% stepper %}
{% step %}
### Install

```sh
rokit add Proton-Utilities/ProCMP pcmp
```
{% endstep %}

{% step %}
### Scaffold

```sh
pcmp init
```

Writes `pcmp.json5` and a JSON Schema beside it, holding an `abstract` base profile, a
`dev` profile and a `release` profile.
{% endstep %}

{% step %}
### Build

```sh
pcmp build --var version=v1.0.0
```

```
  built   dev      dist/dev/app.luau      (12 ms)
  built   release  dist/release/app.luau  (14 ms)

2 built, 0 cached, 0 failed
```
{% endstep %}
{% endstepper %}

## What a profile changes

One source file, two profiles, two artifacts.

{% code title="src/init.luau" %}
```lua
--!strict
local VERSION: string = PCMP_VERSION

local function boot(): string
	if DEBUG then
		print("verbose telemetry")
	end
	return VERSION
end

return boot()
```
{% endcode %}

{% tabs %}
{% tab title="release" %}
{% code title="dist/release/app.luau" %}
```lua
-- app v1.0.0
local a='v1.0.0'local function boot()return a end return boot()
```
{% endcode %}
{% endtab %}

{% tab title="dev" %}
{% code title="dist/dev/app.luau" %}
```lua
local VERSION: string = 'v1.0.0'

local function boot(): string
    if true then
        print('verbose telemetry')
    end

    return VERSION
end

return boot()
```
{% endcode %}
{% endtab %}
{% endtabs %}

`DEBUG` is substituted as a value, the condition folds, and the branch leaves the release
artifact rather than shipping as `if false then`.

## Commands

| Command | Purpose |
| --- | --- |
| `pcmp init` | Write a starter manifest and its schema |
| `pcmp plan` | Resolve and print, building nothing |
| `pcmp check` | Lint the manifest and the plan |
| `pcmp build` | Build every task, or a selection |
| `pcmp watch` | Rebuild whenever an input changes |
| `pcmp verify` | Prove the output is reproducible |
| `pcmp explain` | Print the darklua configuration a task compiles to |
| `pcmp schema` | Emit the manifest schema |

{% content-ref url="install.md" %}
[install.md](install.md)
{% endcontent-ref %}

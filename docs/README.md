---
description: One Luau source tree, many build targets, from a single manifest.
icon: layer-group
---

# ProCMP

A minified release, a readable debug build, a Roblox variant and a Lune variant, all described in one file and built by one command.

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is a single binary with nothing to install beside it.

{% hint style="success" %}
**A build stamp costs you nothing.**

Put the version and the time in your header. `pcmp build --lock` writes down what the build read, and `pcmp build --frozen` reproduces it exactly, timestamp included.
{% endhint %}

## What a profile changes

{% code title="src/init.luau" %}
```lua
--!strict
local VERSION: string = PCMP_VERSION

if DEBUG then
	print("verbose telemetry")
end

return VERSION
```
{% endcode %}

{% tabs %}
{% tab title="release" %}
{% code title="dist/release/app.luau" %}
```lua
-- app v1.0.0
local a='v1.0.0'return a
```
{% endcode %}

`DEBUG` folds to `false`, and the branch leaves the artifact instead of shipping as `if false then`.
{% endtab %}

{% tab title="dev" %}
{% code title="dist/dev/app.luau" %}
```lua
local VERSION: string = 'v1.0.0'

if true then
    print('verbose telemetry')
end

return VERSION
```
{% endcode %}

Same source, same manifest, one different profile.
{% endtab %}
{% endtabs %}

## Where to go

{% content-ref url="install.md" %}
[install.md](install.md)
{% endcontent-ref %}

{% content-ref url="first-build.md" %}
[first-build.md](first-build.md)
{% endcontent-ref %}

{% content-ref url="manifest.md" %}
[manifest.md](manifest.md)
{% endcontent-ref %}

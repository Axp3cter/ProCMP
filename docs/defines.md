---
description: Tokens for paths, constants for source.
---

# Vars and defines

Two knobs, one distinction: a **var** names a string used to build a path or a header;
a **define** names a value substituted into your source.

```lua
vars   = { name = "app", channel = "stable" },   -- {name}, {channel}, PCMP_NAME, PCMP_CHANNEL
define = { DEBUG = false, MAX_RETRY = 3 },       -- DEBUG, MAX_RETRY
```

Every var is also a define, because a name worth putting in a path is usually worth
reading from source. The reverse does not hold — a define can be a boolean or a number,
and neither belongs in a filename.

Both can be set from the command line, which is what makes a manifest reusable across
environments it was never written to anticipate:

```sh
pcmp build --var version="$(git describe --tags)" -D CHANNEL=beta
```

## Defines

```lua
release = {
	define = {
		DEBUG     = false,
		MAX_RETRY = 3,
		CHANNEL   = "stable",
	},
}
```

Read them as plain globals:

```lua
if DEBUG then
	print("verbose telemetry")
end

local channel: string = CHANNEL
```

`_G.DEBUG` and `_G["DEBUG"]` work identically. All three are compile-time placeholders —
after injection none of them appears in the artifact, because the read was replaced by
the value itself.

{% hint style="warning" %}
`getgenv().DEBUG` does **not** work. It is a function call, so there is nothing to
replace at build time; it would read a real table at runtime, and the branch would ship.
{% endhint %}

## They are removed, not disabled

Values are injected as AST nodes, so they participate in constant folding.

{% tabs %}
{% tab title="Source" %}
```lua
local VERSION: string = PCMP_VERSION

if DEBUG then
	print("verbose telemetry")
	print("second debug line")
end

return { version = VERSION }
```
{% endtab %}

{% tab title="DEBUG = false" %}
```lua
local a='v1.0.0'return{version=a}
```

The branch is gone — not `if false then`, and not left to the Luau optimiser. Code that
is not shipped cannot be read out of your artifact.
{% endtab %}

{% tab title="DEBUG = true" %}
```lua
local a='v1.0.0'do print('verbose telemetry')print('second debug line')end
return{version=a}
```

The condition folded to a bare block, so nothing is tested at runtime.
{% endtab %}
{% endtabs %}

This needs `compute_expression` to fold the condition and `remove_unused_if_branch` to
drop the branch, in that order. Without them the value is still injected, but the branch
ships. [darklua](darklua.md) covers the ordering.

## Built in

One constant per var, plus `PCMP_PROFILE`, plus one per [matrix](manifest.md#matrix)
axis. A define you write with the same name wins.

```lua
vars = { name = "app", version = "v1.0.0" },
```

```lua
print(PCMP_NAME, PCMP_VERSION, PCMP_PROFILE)   -- "app", "v1.0.0", "release"
```

There is no fixed list beyond `PCMP_PROFILE` — the set is whatever you named.

## Types

A define is a boolean, a finite number, or a string. The type reaches the cache key, so
`true` and `"true"` are different builds. Infinity and NaN are rejected
([`bad-define`](diagnostics.md)) — they have no literal form.

## Values from outside

Three ways in, in order of precedence:

```sh
pcmp build --var version=v1.2.3        # wins outright
```

```lua
vars = { version = pcmp.env("VERSION") },      -- errors when unset
vars = { version = pcmp.envOr("VERSION", "v0.0.0-dev") },   -- explicit fallback
```

`pcmp.env` fails rather than substituting an empty string, so a misconfigured CI job
reports a clear error instead of shipping a blank version. `--var` and `--define` work
whatever the manifest format, so a JSON or TOML project is not stuck with literals.

{% hint style="info" %}
There is no build-timestamp define. A timestamp makes two builds of the same commit
differ, which breaks `pcmp verify` and the cache. Pass one in with
`pcmp.env("BUILD_STAMP")` if you want it, so the decision stays visible in your manifest.
{% endhint %}

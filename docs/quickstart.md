---
description: An empty directory to a built artifact.
---

# Quickstart

{% stepper %}
{% step %}
**Source**

{% code title="src/init.luau" %}
```lua
--!strict
local VERSION: string = PCMP_VERSION

if DEBUG then
	print("verbose telemetry")
end

return { version = VERSION }
```
{% endcode %}

`PCMP_VERSION` and `DEBUG` are not runtime globals. ProCMP replaces them with literals
at build time. See [Vars and defines](defines.md).
{% endstep %}

{% step %}
**Manifest**

{% code title="pcmp.luau" %}
```lua
return {
	vars = { name = "app", version = pcmp.env("VERSION") },

	profiles = {
		release = {
			entry  = "src/init.luau",
			output = "dist/{name}.luau",
			define = { DEBUG = false },

			darklua = {
				generator = "dense",
				rules     = {
					"compute_expression",
					"remove_unused_if_branch",
					"remove_types",
					"remove_comments",
					"rename_variables",
				},
			},
		},
	},
}
```
{% endcode %}
{% endstep %}

{% step %}
**Build**

```sh
pcmp build --env VERSION=v1.0.0
```

```
  built   release  dist/app.luau  (1 ms)

1 built, 0 cached, 0 failed
```
{% endstep %}
{% endstepper %}

## The artifact

{% code title="dist/app.luau" %}
```lua
local a='v1.0.0'return{version=a}
```
{% endcode %}

The version is a literal, and the `if DEBUG` branch is **not in the file**. Injecting
`DEBUG` as a constant made the condition foldable, which made the branch unreachable, so
darklua removed it.

## Other commands

```sh
pcmp plan             # what would be built, without building it
pcmp check            # lint the manifest and the plan
pcmp explain release  # the darklua config this task compiles to
pcmp watch            # rebuild on every change
pcmp verify           # build twice, confirm the bytes match
```

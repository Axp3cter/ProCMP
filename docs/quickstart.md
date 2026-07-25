---
description: An empty directory to a built artifact.
---

# Quickstart

```sh
pcmp init
```

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

{% code title="pcmp.json5" %}
```json5
{
  $schema: "./pcmp.schema.json",

  vars: { name: "app", version: "v0.0.0-dev" },

  profiles: {
    release: {
      entry: "src/init.luau",
      output: "dist/{name}.luau",
      define: { DEBUG: false },

      darklua: {
        generator: "dense",
        rules: [
          "compute_expression",
          "remove_unused_if_branch",
          "remove_types",
          "remove_comments",
          "rename_variables",
        ],
      },
    },
  },
}
```
{% endcode %}

```
$ pcmp build --var version=v1.0.0
  built   release  dist/app.luau  (1 ms)

1 built, 0 cached, 0 failed
```

{% code title="dist/app.luau" %}
```lua
local a='v1.0.0'return{version=a}
```
{% endcode %}

`PCMP_VERSION` and `DEBUG` are not runtime globals. Both were replaced at build time,
and the `if DEBUG` branch is gone rather than shipped and skipped.

{% content-ref url="defines.md" %}
[defines.md](defines.md)
{% endcontent-ref %}

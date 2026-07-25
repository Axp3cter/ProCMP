---
description: Bundle one Luau source tree into many build targets from a single manifest.
---

# ProCMP

One source tree, many artifacts. A minified release, a readable debug build, a Roblox
variant, a Lune variant, all from one manifest.

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is one static binary
with nothing to install alongside it.

```sh
pcmp init
```

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
        rules: ["compute_expression", "remove_unused_if_branch"],
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

`DEBUG` is injected as an AST node, so `if DEBUG then` folds and the branch is gone from
the artifact rather than shipped and skipped.

```sh
pcmp plan     # what would be built
pcmp check    # lint the manifest and the plan
pcmp build    # build it
pcmp watch    # rebuild on every change
pcmp verify   # prove the output is reproducible
```

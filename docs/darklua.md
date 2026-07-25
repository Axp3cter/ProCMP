---
description: darklua's configuration, passed through untouched.
---

# darklua

A profile's `darklua` block is darklua's own configuration format, deserialised by
darklua. An unknown key is an error carrying darklua's message and the JSON that was
emitted.

{% code title="pcmp.json5" %}
```json5
darklua: {
  generator: { name: "dense", column_span: 120 },
  apply_to_files: ["src/**"],
  skip_files: ["**/*.test.luau"],
  lua_extension: "luau",

  bundle: {
    require_mode: { name: "luau", aliases: { pkg: "./Packages" } },
    excludes: ["@lune/**"],
  },

  rules: [
    "compute_expression",
    "remove_unused_if_branch",
    "remove_types",
    "remove_comments",
    "rename_variables",
    { rule: "convert_require", current: "path", target: "roblox" },
  ],
}
```
{% endcode %}

{% content-ref url="https://darklua.com/docs/config/" %}
[darklua configuration reference](https://darklua.com/docs/config/)
{% endcontent-ref %}

## Rules

Three ways to say it, meaning three different things:

<table><thead><tr><th width="200"></th><th></th></tr></thead><tbody>
<tr><td><code>rules</code> omitted</td><td>darklua applies its own default rules</td></tr>
<tr><td><code>rules: []</code></td><td>No rules. Bundle and generate, transform nothing</td></tr>
<tr><td><code>rules: [a, b, c]</code></td><td>Exactly these, in this order</td></tr>
</tbody></table>

A rule in object form takes `apply_to_files` and `skip_files`.

## Injection comes first

Each `define` becomes an `inject_global_value` rule ahead of whatever you wrote.

```
$ pcmp explain release

darklua configuration
  {
    "bundle": { "require_mode": { "name": "luau" } },
    "generator": "dense",
    "rules": [
      { "rule": "inject_global_value", "identifier": "DEBUG", "value": false },
      { "rule": "inject_global_value", "identifier": "PCMP_NAME", "value": "app" },
      { "rule": "inject_global_value", "identifier": "PCMP_PROFILE", "value": "release" },
      "compute_expression",
      "remove_unused_if_branch"
    ]
  }
```

That block is valid `.darklua.json`. Keys come out sorted.

## Order is reported, not corrected

Your order is the order darklua runs. `pcmp check` reports the two orderings darklua's
own documentation calls out, and nothing beyond them:

```
$ pcmp check
error  branch-before-fold: task `release`: `remove_unused_if_branch` runs before `compute_expression`
       help: list every `compute_expression` ahead of `remove_unused_if_branch`
       help:   a branch is only removable once its condition has folded to a constant
```

A rule listed twice is not a finding.

## Headers

`remove_comments` strips Luau directives, and the `dense` and `readable` generators
discard comments regardless. [`header`](manifest.md#entry-and-output) is written after
darklua finishes.

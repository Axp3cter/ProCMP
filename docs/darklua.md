---
description: darklua's own configuration, passed through untouched.
icon: gears
---

# darklua

A profile's `darklua` block is darklua's own configuration format, deserialised by darklua. Nothing is translated, so no capability of the linked version is out of reach.

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
    { rule: "convert_require", current: "path", target: "roblox" },
  ],
}
```
{% endcode %}

{% embed url="https://darklua.com/docs/config/" %}
darklua's configuration reference
{% endembed %}

## Generators

| | |
| --- | --- |
| `retain_lines` | keeps the original line structure, and is darklua's default |
| `dense` | compact, one long line per statement run |
| `readable` | reformatted and indented |

`dense` and `readable` take a `column_span`, either as an object or on their own as a string.

## Rules

Three spellings, three meanings.

| Written | Result |
| --- | --- |
| `rules` omitted | darklua applies its own defaults |
| `rules: []` | no rules, so bundle and generate and transform nothing |
| `rules: [a, b]` | exactly these, in this order |

Each `define` becomes an `inject_global_value` rule placed ahead of whatever you wrote, so later rules see the substituted value.

```
$ pcmp plan release
darklua configuration
  {
    "bundle": { "require_mode": "luau" },
    "generator": "dense",
    "rules": [
      { "rule": "inject_global_value", "identifier": "DEBUG", "value": false },
      { "rule": "inject_global_value", "identifier": "PCMP_NAME", "value": "app" },
      "compute_expression"
    ]
  }
```

{% hint style="success" %}
That block is a valid `.darklua.json`. Keys come out sorted, so two runs print the same thing.
{% endhint %}

## Merging

`darklua` merges key by key down an `extends` chain, so a profile can set `generator` without restating `bundle`.

| | |
| --- | --- |
| object | merges recursively |
| array | replaces outright |
| scalar | replaces |
| `null` | unsets the inherited key |

```json5
unbundled: { extends: "release", darklua: { bundle: null } }
```

Without `null` a child could never clear an inherited `bundle`.

## Rule order

Two orderings are checked, both of them darklua's own.

| Code | Meaning |
| --- | --- |
| `fold-before-inject` | `compute_expression` scheduled before `inject_global_value` |
| `branch-before-fold` | `remove_unused_if_branch` scheduled before `compute_expression` |

Nothing beyond those, because a lint stricter than the tool it lints for would reject working manifests.

{% hint style="info" %}
**Empty tables in a Luau manifest**

An empty Luau table is both an empty list and an empty map, and Luau has no syntax to say which.

`pcmp` reads `{}` as an empty list at the keys darklua treats as lists, which are `rules`, `apply_to_files`, `skip_files`, `excludes` and `globals`. Everywhere else it stays a map, so `aliases = {}` still means no aliases.

In a data format the question does not arise, because you write `[]` or `{}` and mean it.
{% endhint %}

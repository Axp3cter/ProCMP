---
title: darklua
description: darklua's own configuration, passed through untouched.
---

# darklua

A profile's `darklua` block is darklua's own configuration format, deserialised by darklua. Nothing is translated, so no capability of the linked version is out of reach.

```json5 title="pcmp.json5"
darklua: {
  generator: { name: "dense", column_span: 120 },
  apply_to_files: ["src/**"], // (1)!
  skip_files: ["**/*.test.luau"],
  lua_extension: "luau",

  bundle: {
    require_mode: { name: "luau", aliases: { pkg: "./Packages" } },
    excludes: ["@lune/**"],
  },

  rules: [
    "compute_expression",
    { rule: "convert_require", current: "path", target: "roblox" }, // (2)!
  ],
}
```

1.  These match each file's path relative to the entry, so `src/**` matches nothing when the entry is already `src/init.luau`. A filter that matches nothing is the usual cause of [`no-output`](diagnostics.md#no-output).
2.  A rule is a bare name, or an object with a `rule` key and that rule's own settings.

The full reference is at [darklua.com/docs/config](https://darklua.com/docs/config/).

## Generators

| Generator | Output |
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

```console
$ pcmp plan release
darklua
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

That block is a valid `.darklua.json`. Keys come out sorted, so two runs print the same thing and a diff of the two says something.

## Merging

`darklua` merges key by key down an `extends` chain, so a profile can set `generator` without restating `bundle`.

| Inherited value | What a child does to it |
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

| Code | What it catches |
| --- | --- |
| [`fold-before-inject`](diagnostics.md#fold-before-inject) | `compute_expression` before `inject_global_value`, so folding cannot see the substituted value and the define does nothing |
| [`branch-before-fold`](diagnostics.md#branch-before-fold) | `remove_unused_if_branch` before `compute_expression`, so the condition is not a constant yet and the branch survives |

Nothing beyond those, because a lint stricter than the tool it lints for would reject working manifests.

## Empty tables in a Luau manifest

An empty Luau table is both an empty list and an empty map, and Luau has no syntax to say which.

`pcmp` reads `{}` as an empty list at the keys darklua treats as lists, which are `rules`, `apply_to_files`, `skip_files`, `excludes` and `globals`. Everywhere else it stays a map, so `aliases = {}` still means no aliases.

In a data format the question does not arise, because you write `[]` or `{}` and mean it.

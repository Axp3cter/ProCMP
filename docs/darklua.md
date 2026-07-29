---
description: darklua's configuration, passed through untouched.
---

# darklua

A profile's `darklua` block is darklua's own configuration format, deserialised by
darklua. Nothing is translated, so no capability of the linked version is out of reach.

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

See darklua's [configuration reference](https://darklua.com/docs/config/).

## Generators

| | |
| --- | --- |
| `retain_lines` | keeps the original line structure. darklua's default |
| `dense` | compact, one long line per statement run |
| `readable` | reformatted and indented |

`dense` and `readable` take a `column_span`, either as an object or on their own as a
string.

## Rules

Three spellings, three meanings:

| | |
| --- | --- |
| `rules` omitted | darklua applies its own defaults |
| `rules: []` | no rules — bundle and generate, transform nothing |
| `rules: [a, b]` | exactly these, in this order |

Each `define` becomes an `inject_global_value` rule placed ahead of whatever you wrote, so
later rules see the substituted value. `pcmp plan <TASK>` prints the result, and that block
is a valid `.darklua.json`.

{% hint style="info" %}
An empty Luau table is both an empty list and an empty map, and Luau has no syntax to say
which. `pcmp` reads `{}` as an empty list at the keys darklua treats as lists — `rules`,
`apply_to_files`, `skip_files`, `excludes`, `globals` — and as an empty map everywhere
else, so `aliases = {}` still means no aliases. In a data format the question does not
arise: write `[]` or `{}` and mean it.
{% endhint %}

## Merging

`darklua` merges key by key down an `extends` chain, so a profile can set `generator`
without restating `bundle`. Arrays replace outright — a child listing `rules` means exactly
those — and `null` unsets an inherited key:

```json5
unbundled: { extends: "release", darklua: { bundle: null } }
```

Two orderings are checked, both of them darklua's own: every `inject_global_value` must
precede every `compute_expression`, and every `compute_expression` must precede every
`remove_unused_if_branch`. Nothing beyond those, because a lint stricter than the tool it
lints for would reject working manifests.

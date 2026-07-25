---
description: darklua's configuration, passed through untouched.
---

# darklua

A profile's `darklua` block is darklua's own configuration format, deserialised by
darklua. Nothing is translated, so no capability of the linked version is out of reach.

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

## Generators

| `generator` | |
| --- | --- |
| `retain_lines` | Keeps the original line structure. The darklua default |
| `dense` | Compact, one long line per statement run |
| `readable` | Reformatted and indented |

`dense` and `readable` take a `column_span`, either as `{ name: "dense", column_span: 120 }`
or on their own as `"dense"`.

## Rules

Three ways to say it, meaning three different things:

| Written | Result |
| --- | --- |
| `rules` omitted | darklua applies its own default rules |
| `rules: []` | No rules. Bundle and generate, transform nothing |
| `rules: [a, b, c]` | Exactly these, in this order |

A rule in object form takes `apply_to_files` and `skip_files` of its own.

### Injection comes first

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

{% hint style="success" %}
That block is a valid `.darklua.json`. Keys come out sorted, so two runs of `explain`
on one manifest print the same bytes.
{% endhint %}

### Order is reported, not corrected

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
darklua finishes, so `--!native` survives either one.

## When darklua rejects the block

An unknown key or a bad value is a task failure carrying darklua's own message and the
JSON that was emitted, so there is no guessing at what ProCMP sent.

```
$ pcmp build
  FAILED  release  dist/a.luau  (1 ms)
          task `release` emitted a darklua configuration darklua rejected
            unknown field `generatorr`
          {
            "generatorr": "dense",
            "rules": [
              {
                "rule": "inject_global_value",
                "identifier": "PCMP_PROFILE",
                "value": "release"
              }
            ]
          }

0 built, 0 cached, 1 failed
```

{% content-ref url="cli.md" %}
[cli.md](cli.md)
{% endcontent-ref %}

---
description: darklua's configuration, passed through untouched.
---

# darklua

A profile's `darklua` block is darklua's own configuration format. ProCMP does not
model it, rename it, restrict it or reorder it. The block is deserialised by darklua
itself, so the accepted set is exactly what the linked version supports.

{% code title="pcmp.luau" %}
```lua
darklua = {
	generator      = { name = "dense", column_span = 120 },
	apply_to_files = { "src/**" },
	skip_files     = { "**/*.test.luau" },
	lua_extension  = "luau",

	bundle = {
		require_mode = { name = "luau", aliases = { pkg = "./Packages" } },
		excludes     = { "@lune/**" },
	},

	rules = {
		"compute_expression",
		"remove_unused_if_branch",
		"remove_types",
		"remove_comments",
		"rename_variables",
		{ rule = "convert_require", current = "path", target = "roblox" },
	},
},
```
{% endcode %}

{% content-ref url="https://darklua.com/docs/config/" %}
[darklua configuration reference](https://darklua.com/docs/config/)
{% endcontent-ref %}

A key darklua does not know is an error carrying darklua's own message and the JSON that
was emitted:

```
$ pcmp build
  FAILED  a  dist/a.luau  (0 ms)
          task `a` emitted a darklua configuration darklua rejected
            unknown field `nonsense`
          {
            "nonsense": true,
            "rules": [
```

## Rules

Three ways to say it, and they mean three different things:

<table><thead><tr><th width="200"></th><th></th></tr></thead><tbody>
<tr><td><code>rules</code> omitted</td><td>darklua applies its own default rules.</td></tr>
<tr><td><code>rules = {}</code></td><td>No rules. Bundle and generate, transform nothing.</td></tr>
<tr><td><code>rules = { a, b, c }</code></td><td>Exactly these, in this order.</td></tr>
</tbody></table>

```
$ pcmp plan
  release  dist/app.luau  darklua defaults
```

A rule in object form takes `apply_to_files` and `skip_files`, so one rule can be scoped
without splitting the profile:

```lua
rules = {
	"compute_expression",
	{ rule = "remove_comments", skip_files = "**/vendor/**" },
	{ rule = "rename_variables", apply_to_files = "src/**/*.luau" },
},
```

## Injection comes first

Each [`define`](defines.md) becomes an `inject_global_value` rule, placed ahead of
whatever you wrote. Nothing downstream can fold a value that has not been substituted
yet, so this is the only position that works.

```
$ pcmp explain a

darklua configuration
  {
    "apply_to_files": [ "src/**" ],
    "bundle": { "require_mode": { "name": "luau" } },
    "generator": "dense",
    "lua_extension": "luau",
    "rules": [
      { "rule": "inject_global_value", "identifier": "DEBUG", "value": false },
      { "rule": "inject_global_value", "identifier": "PCMP_NAME", "value": "app" },
      { "rule": "inject_global_value", "identifier": "PCMP_PROFILE", "value": "a" },
      "compute_expression",
      "remove_unused_if_branch"
    ]
  }
```

That block is valid `.darklua.json`. Paste it into a config file and darklua on its own
produces the same artifact.

{% hint style="info" %}
Keys come out sorted. A Luau table iterates in hash order, so an unsorted block would
serialise differently on each run and give the same configuration two cache keys.
{% endhint %}

## Order is reported, not corrected

Your order is the order darklua runs. `pcmp check` reports the two orderings darklua's
own documentation calls out, and nothing beyond them:

```
$ pcmp check
error  branch-before-fold: task `release`: `remove_unused_if_branch` runs before `compute_expression`
       help: list every `compute_expression` ahead of `remove_unused_if_branch`
       help:   a branch is only removable once its condition has folded to a constant
```

darklua's own default rule list is a valid ordering. A lint stricter than the tool it
lints for would reject working manifests, so there are only these two.

{% hint style="info" %}
A rule listed twice is not a finding. Running a pass again after an earlier rule has
exposed new foldable code is a technique, not a mistake.
{% endhint %}

## Headers

`remove_comments` strips Luau directives along with everything else, and the `dense` and
`readable` generators discard comments regardless. Headers are therefore written by
ProCMP after darklua finishes, so nothing downstream can remove them.

{% code title="pcmp.luau" %}
```lua
header = {
	"--!native",
	"--!optimize 2",
	"-- {name} {version}, generated, do not edit",
},
```
{% endcode %}

{% code title="dist/release/app.luau" %}
```lua
--!native
--!optimize 2
-- app v0.0.0-dev, generated, do not edit
local a={}a.size=16 return a
```
{% endcode %}

Tokens are expanded. With a directory `output` every generated file gets the header.

---
description: Install pcmp and wire up editor completion.
---

# Install

{% tabs %}
{% tab title="Binary" %}
Download the archive for your platform from
[releases](https://github.com/Proton-Utilities/ProCMP/releases), extract it, and put
`pcmp` on your `PATH`.

Each release ships `SHA256SUMS`:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```
{% endtab %}

{% tab title="Cargo" %}
```sh
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

Needs Rust 1.90+. The first build takes a few minutes — darklua and a Luau interpreter
are compiled in.
{% endtab %}
{% endtabs %}

```sh
pcmp --version
```

## Editor completion

`pcmp schema` emits type definitions from the same types the parser uses, so they cannot
describe something ProCMP will reject.

{% tabs %}
{% tab title="Luau manifests" %}
```sh
pcmp schema --format luau > pcmp.d.luau
```

{% code title=".vscode/settings.json" %}
```json
{ "luau-lsp.types.definitionFiles": ["pcmp.d.luau"] }
```
{% endcode %}

You get field completion on every ProCMP field and a type error on `output = 42` before
you run a build. The `pcmp` global is declared too. The `darklua` block is typed open,
because darklua owns that vocabulary and validates it itself.
{% endtab %}

{% tab title="JSON and TOML manifests" %}
```sh
pcmp schema > pcmp.schema.json
```

```json
{
  "$schema": "./pcmp.schema.json",
  "project": { "name": "app" }
}
```

The schema sets `additionalProperties: false`, so a typo'd key is flagged as you type.
{% endtab %}
{% endtabs %}

Both files are safe to commit. Regenerate after upgrading ProCMP.

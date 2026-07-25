---
description: Install pcmp, then wire up editor completion.
---

# Install

{% tabs %}
{% tab title="Rokit" %}
```sh
rokit add Proton-Utilities/ProCMP pcmp
```

The trailing `pcmp` is the alias. Without it the command is named after the repository.

{% code title="rokit.toml" %}
```toml
[tools]
pcmp = "Proton-Utilities/ProCMP@4.0.0"
```
{% endcode %}
{% endtab %}

{% tab title="Aftman" %}
```sh
aftman add Proton-Utilities/ProCMP pcmp
```

{% code title="aftman.toml" %}
```toml
[tools]
pcmp = "Proton-Utilities/ProCMP@4.0.0"
```
{% endcode %}
{% endtab %}

{% tab title="Cargo" %}
```sh
cargo install --locked --git https://github.com/Proton-Utilities/ProCMP
```

Needs Rust 1.90 or newer. The first build takes a few minutes, because darklua and a
Luau interpreter are compiled in.

{% hint style="warning" %}
Keep `--locked`. Without it Cargo resolves dependencies to their latest versions, and
one of those raises its minimum compiler past the version this release pins.
{% endhint %}
{% endtab %}

{% tab title="Binary" %}
Download the archive for your platform from
[releases](https://github.com/Proton-Utilities/ProCMP/releases) and put `pcmp` on your
`PATH`. Every release ships `SHA256SUMS`:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```
{% endtab %}
{% endtabs %}

## Editor completion

`pcmp init` writes the schema beside the manifest, so a fresh project needs nothing
further. Regenerate after upgrading:

{% tabs %}
{% tab title="JSON, JSONC, JSON5 and TOML" %}
```sh
pcmp schema > pcmp.schema.json
```

Point the manifest at it:

{% code title="pcmp.json5" %}
```json5
{ $schema: "./pcmp.schema.json" }
```
{% endcode %}
{% endtab %}

{% tab title="Luau" %}
```sh
pcmp schema --format luau > pcmp.d.luau
```

{% code title=".vscode/settings.json" %}
```json
{ "luau-lsp.types.definitionFiles": ["pcmp.d.luau"] }
```
{% endcode %}
{% endtab %}
{% endtabs %}

Both are generated from the same type the parser uses, so neither can describe something
ProCMP would reject.

{% content-ref url="manifest.md" %}
[manifest.md](manifest.md)
{% endcontent-ref %}

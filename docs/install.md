---
description: Install pcmp and wire up editor completion.
---

# Install

## Rokit

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

## Aftman

```sh
aftman add Proton-Utilities/ProCMP pcmp
```

{% code title="aftman.toml" %}
```toml
[tools]
pcmp = "Proton-Utilities/ProCMP@4.0.0"
```
{% endcode %}

## Cargo

```sh
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

Needs Rust 1.90 or newer. The first build takes a few minutes, because darklua and a
Luau interpreter are compiled in.

## Binary

Download the archive for your platform from
[releases](https://github.com/Proton-Utilities/ProCMP/releases) and put `pcmp` on your
`PATH`. Each release ships `SHA256SUMS`:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

## Editor completion

`pcmp schema` emits definitions from the types the parser uses, so they cannot describe
something ProCMP will reject. Both files are safe to commit.

```sh
pcmp schema --format luau > pcmp.d.luau   # Luau manifests
pcmp schema > pcmp.schema.json            # JSON and TOML manifests
```

{% code title=".vscode/settings.json" %}
```json
{ "luau-lsp.types.definitionFiles": ["pcmp.d.luau"] }
```
{% endcode %}

{% code title="pcmp.json5" %}
```json5
{
  $schema: "./pcmp.schema.json",
  vars: { name: "app" },
}
```
{% endcode %}

`pcmp init` writes both the manifest and its schema, so a fresh project already has
this. `pcmp init --format luau` writes `pcmp.luau` and `pcmp.d.luau` instead.

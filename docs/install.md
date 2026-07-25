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

## Cargo

```sh
cargo install --git https://github.com/Proton-Utilities/ProCMP
```

Needs Rust 1.90 or newer. The first build takes a few minutes, because darklua and a
Luau interpreter are compiled in.

## Binary

Download the archive for your platform from
[releases](https://github.com/Proton-Utilities/ProCMP/releases) and put `pcmp` on your `PATH`.
Each release ships `SHA256SUMS`:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

## Editor completion

`pcmp init` writes the schema beside the manifest, so a fresh project already has this.
Regenerate after upgrading:

```sh
pcmp schema > pcmp.schema.json            # JSON, JSONC, JSON5 and TOML
pcmp schema --format luau > pcmp.d.luau   # Luau
```

{% code title=".vscode/settings.json" %}
```json
{ "luau-lsp.types.definitionFiles": ["pcmp.d.luau"] }
```
{% endcode %}

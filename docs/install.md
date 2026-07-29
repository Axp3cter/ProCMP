---
description: Getting the binary, and teaching your editor about the manifest.
icon: box-open
---

# Install

{% tabs %}
{% tab title="rokit" %}
```sh
rokit add Proton-Utilities/ProCMP
```
{% endtab %}

{% tab title="aftman" %}
```sh
aftman add Proton-Utilities/ProCMP
```
{% endtab %}

{% tab title="cargo" %}
```sh
cargo install --locked --git https://github.com/Proton-Utilities/ProCMP
```
{% endtab %}
{% endtabs %}

The binary is called `pcmp`. darklua is linked in, so there is no second tool to install and no version to keep in step.

```sh
pcmp --version
```

```
4.0.0
darklua 0.19.0
```

## Editor completion

{% tabs %}
{% tab title="Data manifest" %}
```sh
pcmp schema > pcmp.schema.json
```

Point at it from the manifest itself.

```json5
{ $schema: "./pcmp.schema.json" }
```
{% endtab %}

{% tab title="Luau manifest" %}
```sh
pcmp schema --format luau > pcmp.d.luau
```

`luau-lsp` picks it up, including the `pcmp` globals.
{% endtab %}
{% endtabs %}

{% hint style="info" %}
`pcmp init` writes neither of these.

A generated file committed to a repository goes stale on the next upgrade with nothing to notice, so generating one is a choice you make rather than a default you inherit. If you do commit one, `pcmp check` reports `stale-schema` when it stops matching.
{% endhint %}

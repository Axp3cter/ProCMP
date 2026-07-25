---
description: Bundle one Luau source tree into many build targets from a single manifest.
---

# ProCMP

One source tree, many artifacts. A minified release, a readable debug build, a Roblox
variant, a Lune variant, all from one manifest.

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is one static binary
with nothing to install alongside it.

```sh
pcmp plan     # what would be built
pcmp check    # lint the manifest and the plan
pcmp build    # build it
pcmp watch    # rebuild on every change
pcmp verify   # prove the output is reproducible
```

{% content-ref url="quickstart.md" %}
[quickstart.md](quickstart.md)
{% endcontent-ref %}

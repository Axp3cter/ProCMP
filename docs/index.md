---
title: ProCMP
description: One Luau source tree, many build targets, from a single manifest.
---

# ProCMP

A minified release, a readable debug build, a Roblox variant and a Lune variant, all described in one file and built by one command.

[darklua](https://darklua.com) is linked in as a library, so `pcmp` is a single binary with nothing to install beside it.

## What a profile changes

```lua title="src/init.luau"
--!strict
local VERSION: string = PCMP_VERSION

if DEBUG then
	print("verbose telemetry")
end

return VERSION
```

=== "release"

    ```lua title="dist/release/app.luau"
    -- app v1.0.0
    local a='v1.0.0'return a
    ```

    `DEBUG` folds to `false`, and the branch leaves the artifact instead of shipping as `if false then`.

=== "dev"

    ```lua title="dist/dev/app.luau"
    local VERSION: string = 'v1.0.0'

    if true then
        print('verbose telemetry')
    end

    return VERSION
    ```

    Nothing was stripped, so a stack trace still points at the line you wrote.

One source file, one manifest, two profiles. The header carries a version and can carry a timestamp, because `pcmp build --lock` writes down what the build read and `pcmp build --frozen` reproduces it exactly.

## Where to go

[Install](install.md)

:   One binary from rokit, aftman or cargo, and editor completion for the manifest.

[Your first build](first-build.md)

:   From an empty project to two artifacts and a cache that holds.

[Manifest](manifest.md)

:   Every field, what it does, and the five formats it can be written in.

[CLI](cli.md)

:   Selecting tasks, reading a build, and what an exit code means.

[Diagnostics](diagnostics.md)

:   Every code `pcmp` can report, generated from the binary itself.

# Verse LSP Community Edition

Alternative LSP server for [Verse](https://dev.epicgames.com/documentation/en-us/fortnite/verse-language-reference), built on top of the official Verse implemention in Unreal Engine for baseline parity.

This exists because the official LSP server lags and lacks features, without ability to be improved by the community. If that situation improves significantly, this project will cease to be maintained.

**WORK IN PROGRESS**

- [x] Diagnostics
- [ ] Next Up: Completions

## License

Code in this repository is MIT licensed.

The [Unreal® Engine EULA](https://www.unrealengine.com/en-US/eula/unreal) is applicable to development, distribution, and usage.

## How to compile

**Prerequisites:**

* [Rust](https://rust-lang.org/) toolchain
* 150 GB disk space
* (Windows) Visual Studio C++ Tools: MSVC and Windows 10/11 SDK

**Build Steps:**

1. Get a local copy of [UnrealEngine source code](https://www.unrealengine.com/en-US/ue-on-github)¹ on `ue5-main` branch
2. Run top-level scripts to setup build tools
3. Clone this repository under `<UE dir>/Engine/Source/Programs/`
4. (Optional) Within this cloned repository:
   ```bash
   cargo build -p verse_lsp_rs --release
   ```
   This should be done automatically by the pre-build step in VerseLspCE module.
5. Within UE source directory:
   ```bash
   # Windows
   ./Engine/Build/BatchFiles/RunUBT.bat -Mode=Build VerseLspCE Win64 Shipping
   ```
   ```bash
   # Linux
   ./Engine/Build/BatchFiles/RunUBT.sh -Mode=Build VerseLspCE Linux Shipping
   ```
6. The binary is at `Engine/Binaries/<platform>/VerseLspCE-<platform>-Shipping[.exe]`

¹ Tip: Cloning with `--depth 1` can save you some disk space and download time

## Development

A convenient way to debug is to start the server in TCP mode through `gdb`.

```bash
./Engine/Build/BatchFiles/RunUBT.sh -Mode=Build VerseLspCE Linux Development

gdb --args ./Engine/Binaries/Linux/VerseLspCE --tcp 127.0.0.1:9010
```

Then configure the client to connect to that address. Example for neovim 0.11+:

```lua
-- lsp/verse.lua
...
cmd = function(dispatchers)
    return vim.lsp.rpc.connect("127.0.0.1", 9010)(dispatchers)
end,
...
```

## Note about Rust

The LSP server is written in Rust, delegating to the Verse cpp bridge (UE module).
Ideally, the UE module should have been built as a static library instead of the other way round, but this isn't supported by Unreal Build Tool.  
Also unsupported is the ability to use [cxx](https://github.com/dtolnay/cxx), because I couldn't make UBT work with the proper `gnu++` standard.


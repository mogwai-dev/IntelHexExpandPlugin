# IntelHexExpandPlugin (x64 only)

WinMerge `FILE_PACK_UNPACK` plugin that expands Intel HEX (`.hex`, `.ihx`) into address + byte lines for diff readability.

## Scope

- x64 only (`x86_64-pc-windows-msvc`)
- Unpack only (`PackFile` is intentionally not supported)

## Build

Prerequisites:

- Rust toolchain (`stable-x86_64-pc-windows-msvc`)
- Visual Studio Build Tools (C++)
- Windows SDK (with `midl.exe`)

Build command:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Output DLL:

- `target/x86_64-pc-windows-msvc/release/intel_hex_expand.dll`

## Install to WinMerge

Copy DLL into one of:

- `MergePlugins/` next to `WinMerge.exe`
- `%APPDATA%/WinMerge/MergePlugins/`
- `%USERPROFILE%/Documents/WinMerge/MergePlugins/`

No COM registration is required.

## Plugin behavior

- `PluginEvent`: `FILE_PACK_UNPACK`
- `PluginFileFilters`: `\.hex$;\.ihx$`
- Menu caption: `Expand Intel HEX`

## GitHub release

Recommended release asset name:

- `IntelHexExpandPlugin-x64.zip` (contains `IntelHexExpand.dll` or `intel_hex_expand.dll`)

This repository includes GitHub Actions workflow for x64 build artifact generation.

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

Download and run `IntelHexExpandPlugin-x64-setup.exe` from the latest GitHub Release.
The installer requires no administrator privileges and installs the plugin for the
current user. Close WinMerge before installing or uninstalling the plugin.

For unattended installation (including WinGet), use:

```powershell
IntelHexExpandPlugin-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
```

Uninstall from **Settings > Apps > Installed apps**, or with WinGet after the
package is published.

### Manual install

Copy DLL into one of:

- `MergePlugins/` next to `WinMerge.exe`
- `%APPDATA%/WinMerge/MergePlugins/`
- `%USERPROFILE%/Documents/WinMerge/MergePlugins/`

No COM registration is required.

## Plugin behavior

- `PluginEvent`: `FILE_PACK_UNPACK`
- `PluginFileFilters`: `\.hex$;\.ihx$`
- Menu caption: `Expand Intel HEX`
- Automatic execution by extension is not guaranteed in all WinMerge environments.
- Recommended operation: run plugin manually from WinMerge menu when comparing `.hex` / `.ihx` files.

### Recommended usage (manual)

1. Open two Intel HEX files in WinMerge.
2. Run this plugin from WinMerge's plugin menu (unpack plugin).
3. Compare expanded address+byte output.

## GitHub release

Recommended release asset name:

- `IntelHexExpandPlugin-x64.zip` (contains `IntelHexExpand.dll` or `intel_hex_expand.dll`)

This repository includes GitHub Actions workflow for x64 build artifact generation.

### Create a release

1. Commit and push latest changes.
2. Create and push a version tag:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

3. GitHub Actions will:
	- build x64 DLL
	- create `IntelHexExpandPlugin-x64.zip`
	- create `IntelHexExpandPlugin-x64-setup.exe`
	- publish a GitHub Release for that tag with the DLL, zip, and installer attached

4. Open GitHub repository page:
	- `Actions` tab: confirm workflow succeeded.
	- `Releases` page: confirm new release `vX.Y.Z` exists.

5. (Optional) Edit release notes/title from GitHub UI.

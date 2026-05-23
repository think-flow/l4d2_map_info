# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`map_info` is a terminal UI application for browsing Left 4 Dead 2 `.vpk` (Valve Pak) addon files. It displays a scrollable file list with fuzzy search, and renders the `missions/*.txt` content of selected VPK files.

## Architecture

Two-tier native interop architecture:

1. **`lib/`** — C# NativeAOT library (`VpkInfo.csproj`) that produces a stripped native shared library (`vpkinfo.dll` / `vpkinfo.so`). Uses the `ValvePak` NuGet package to parse VPK files and exposes C-ABI functions via `[UnmanagedCallersOnly]`. The `Vpk` class reads `addoninfo.txt` and `missions/*.txt` entries from a VPK file. `NativeExports` wraps it with `GCHandle`-based lifetime management and UTF-8 string marshaling.

2. **`cli/`** — Rust TUI binary (`ratatui` + `crossterm`) that FFI-binds to the native library. Discovers `.vpk` files, presents a file list with fuzzy filtering, and renders scrollable mission content. The `App` struct owns all UI state.

### Key source files

| File | Role |
|---|---|
| `cli/src/main.rs` | Entry point: installs `color_eyre`, creates `App`, runs the ratatui event loop |
| `cli/src/lib.rs` | `get_vpks()` — discovers `.vpk` files in the working directory (or hardcoded L4D2 addons path in debug builds), sorted by creation time |
| `cli/src/app.rs` | `App` struct definition, `init()`, main `draw()` layout (outer vertical + inner horizontal split) |
| `cli/src/app/event_handler.rs` | Keyboard input handling with a `PREVENT`/`ALLOW` event propagation pattern. Two-phase: list handler then content handler |
| `cli/src/app/list_layout.rs` | File list rendering with fuzzy matching via Jaro-Winkler + Levenshtein hybrid scoring |
| `cli/src/app/content_layout.rs` | Mission content rendering with scroll state management, filter dialog overlay |
| `cli/src/vpk.rs` | `VPKInfo` — safe Rust wrapper around the C# native library FFI, with `Send` impl |
| `cli/build.rs` | Copies the native library (`vpkinfo.dll`/`.so`) from `cli/libs/` into the cargo output directory |
| `lib/Vpk.cs` | C# VPK reader + `NativeExports` FFI surface |

### Data flow

```
.vpk files on disk
  → get_vpks() discovers files → IndexMap<String, PathBuf>
  → App renders list with fuzzy filter (input_state)
  → User selects file → VPKInfo::new(path) calls CreateVk FFI
  → get_mission() calls GetMissionContent FFI
  → Content rendered in scrollable Paragraph
```

### FFI surface (C# → Rust)

The Rust side (`cli/src/vpk.rs`) binds to: `CreateVk`, `DestroyVk`, `GetAddonInfoContent`, `GetMissionContent`, `GetLastErrorMessage`, `FreeString`. Return code `-1` means error (call `GetLastErrorMessage`). Return code `0` with null content pointer means the entry doesn't exist in the VPK.

## Build & run

**Prerequisites:** Rust (edition 2024), .NET 9 SDK, `dotnet` on PATH.

```bash
# Build the C# native library (Windows)
./build_lib.ps1

# Build the C# native library (Linux)
./build_lib.sh

# Build and run the Rust TUI (debug)
cd cli && cargo run

# Build release (optimized binary)
cd cli && cargo build --release
```

The build script `build_lib.ps1`/`.sh` runs `dotnet publish -c Release` for the target platform, then copies the output native library to `cli/libs/`. The cargo `build.rs` copies it from there into the binary output directory at build time.

In debug builds, `get_vpks()` looks for VPK files in the L4D2 addons directory rather than the current working directory.

## Key controls

| Key | Action |
|---|---|
| `q` | Exit |
| `s` | Open file filter (fuzzy search) |
| `Ctrl+X` | Clear filter |
| `↑`/`↓` | Navigate file list |
| `→` | View selected file's mission content |
| `←` | Return to file list |
| `Ctrl+←`/`→` | Resize list/content split |
| `PgUp`/`PgDn`/`Home`/`End` | Scroll content |

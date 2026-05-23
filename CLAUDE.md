# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`map_info` is a terminal UI application for browsing Left 4 Dead 2 `.vpk` (Valve Pak) addon files. It displays a scrollable file list with fuzzy search (by filename or by coop map name), duplicate map detection, and renders the `missions/*.txt` content of selected VPK files. It also supports sorting by creation or modification time.

## Architecture

Pure Rust binary — no external native dependencies. All VPK parsing, KeyValues parsing, and TUI rendering are implemented directly in Rust.

### Architecture: Screen-driven dispatch

The UI uses a `Screen` enum to manage application state. Event handling and rendering dispatch based on the current screen, making it straightforward to add new dialogs or views:

```rust
enum Screen {
    FileList,             // Default: file list with optional filter
    Content,              // Viewing mission content
    Filter(FilterMode),   // Filter dialog (by filename or map name)
    Duplicates,           // Duplicate detection dialog
    MapCodes,             // Map codes list for selected file
}
```

### Key source files

| File | Role |
|---|---|
| `cli/src/main.rs` | Entry point: installs `color_eyre`, creates `App`, runs the ratatui event loop |
| `cli/src/lib.rs` | Module declarations; `get_vpks()` — discovers `.vpk` files, returns file list + timestamp maps |
| `cli/src/app.rs` | `App` struct definition, `run()`/`init()`, background scan via `mpsc` channel, top-level `draw()` layout, `find_duplicates()`, `show_toast()` |
| `cli/src/app/screen.rs` | `Screen` and `FilterMode` enums — single source of truth for app state transitions |
| `cli/src/app/list_state.rs` | `FileListState` — wraps ratatui `ListState` with page-aware scrolling and line-count tracking |
| `cli/src/app/content_state.rs` | `ContentState` — mission content scroll state management |
| `cli/src/app/filter_state.rs` | `FilterState` — filter input string + mode |
| `cli/src/app/sort_state.rs` | `SortState` — sort field (`CreatedAt`/`ModifiedAt`) + ascending/descending toggle |
| `cli/src/app/event_handler.rs` | Keyboard input handling — non-blocking poll (33ms), global keys first, then dispatches by `Screen` |
| `cli/src/app/list_layout.rs` | File list rendering with fuzzy matching via Jaro-Winkler + Levenshtein hybrid scoring + substring position bonus, supports both filename and map-name filtering, highlights matched characters in yellow |
| `cli/src/app/content_layout.rs` | Right-pane rendering: mission content view, filter dialog overlay, duplicates dialog, map codes dialog |
| `cli/src/vpk.rs` | Pure Rust VPK version 1 parser — validates signature (`0x55AA1234`), parses directory tree, reads entries. Single-file VPK only |
| `cli/src/mission.rs` | Valve KeyValues parser — tokenizes mission content with `//` comment support, exposes `parse_coop_maps()` to extract `modes -> coop -> Map` values |

### Data flow

```
.vpk files on disk
  → get_vpks() discovers files → IndexMap<String, PathBuf> + timestamp maps
  → App::init() populates file list, sorts by creation time descending
  → start_background_scan() spawns thread to scan all VPKs for coop maps
  → Background thread sends ScanEvent::Progress / MapEntry / Complete via mpsc channel
  → Main thread drains events each frame via process_scan_events()
  → map_cache populated in real-time, progress bar updates on status bar
  → App draws list (left pane) + right pane by Screen
  → User selects file → VPKInfo::new(path) + get_mission() reads mission content
  → Content rendered in scrollable Paragraph
```

### Background scan

On startup, a background thread scans all VPK files to extract coop map codes. Results are sent via `mpsc::channel` as `ScanEvent` messages. The main thread drains these in `process_scan_events()` before each frame draw. A `Gauge` progress bar is shown on the bottom-right of the status bar during scanning.

### Toast notifications

`App::show_toast(msg)` sets a timed notification that auto-disappears after 2 seconds. Rendered as a dark-background overlay above the status bar. Used for copy confirmation, filter-clear feedback, etc.

## Build & run

**Prerequisites:** Rust (edition 2024).

```bash
cd cli && cargo run          # debug (uses L4D2 addons directory)
cd cli && cargo build --release  # release (uses current working directory)
```

In debug builds, `get_vpks()` looks for VPK files in the L4D2 addons directory rather than the current working directory.

## Key controls

| Key | Action |
|---|---|
| `q` | Exit |
| `s` | Open filename filter (fuzzy search by filename, matched chars highlighted) |
| `c` | Open map filter (fuzzy search by coop map name) |
| `d` | Show duplicate detection dialog (files sharing same map values) |
| `f` | Show map codes list for selected file (selectable, Enter to copy) |
| `t` | Toggle sort by creation time / modification time |
| `r` | Toggle sort order ascending / descending |
| `Ctrl+X` | Clear current filter |
| `↑`/`↓` | Navigate file list |
| `→` | View selected file's mission content |
| `←` | Return to file list |
| `Ctrl+←`/`→` | Resize list/content split |
| `PgUp`/`PgDn` | Scroll content by page (also scrolls file list by visible height) |
| `Home`/`End` | Scroll content to top/bottom (also navigates file list to first/last) |
| `Enter`/`Esc` | Dismiss dialogs (filter, duplicates, map codes) |

## Cargo.toml highlights

Key dependencies: `color-eyre` (error handling), `crossterm` (terminal), `indexmap` (ordered map), `ratatui` (TUI framework, feature `unstable-rendered-line-info`), `textwrap` (word wrapping), `strsim` (Jaro-Winkler + Levenshtein), `arboard` (clipboard). Release profile uses LTO, size optimization, stripping, and panic=abort.

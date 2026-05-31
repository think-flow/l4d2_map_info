# map_info

A terminal UI application for browsing Left 4 Dead 2 `.vpk` addon files, built in pure Rust.

Browse your addon collection with a dual-pane interface — scrollable file list on the left, mission content on the right. Fuzzy search by filename or coop map name, detect duplicate maps across addons, and copy map codes to clipboard.

## Features

- **VPK parsing** — Pure Rust implementation of the Valve Pak (VPK) version 1 format. No external native dependencies.
- **Fuzzy search** — Search by filename or coop map name using a Jaro-Winkler + Levenshtein hybrid scorer with substring position bonus. Matched characters are highlighted.
- **Duplicate detection** — Identifies VPK files that contain overlapping coop map codes.
- **Map codes view** — Lists all coop map codes for the selected addon. Select and press Enter to copy to clipboard.
- **Mission content viewer** — Renders the parsed `missions/*.txt` KeyValues content with word wrapping.
- **Sorting** — Sort by file creation time or modification time, ascending or descending.
- **Background scanning** — On startup, a background thread (using rayon) scans all VPKs for coop map codes with a progress bar.
- **Toast notifications** — Timed overlay messages for copy confirmation and filter feedback.

## Screenshots

The UI uses a two-pane layout with a status bar:

```
┌──────────────────────────────────────────────────────────────────────┐
│  map_info v0.2.0                                                     │
├────────────────────────────┬─────────────────────────────────────────┤
│ > my_addon.vpk             │  "mission"                              │
│   cool_maps.vpk            │  {                                      │
│   survival_pack.vpk        │    "modes"                              │
│                            │    {                                    │
│                            │      "coop"                             │
│                            │      {                                 │
│                            │        "Map"  "c1m1_hotel"             │
│                            │        "Map"  "c1m2_streets"           │
│                            │      }                                 │
│                            │    }                                   │
│                            │  }                                     │
├────────────────────────────┴─────────────────────────────────────────┤
│  3 addons loaded    [s]earch  [c]oop filter  [d]uplicates  [f] maps │
└──────────────────────────────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust (edition 2024) — install via [rustup](https://rustup.rs/)

### Build

```bash
cd cli
cargo build --release
```

The release binary is heavily optimized for size (LTO, symbol stripping, `opt-level = "s"`, `panic = abort`).

### Run

```bash
# Place the binary in your L4D2 addons directory, or run from there:
cd "C:\Program Files (x86)\Steam\steamapps\common\Left 4 Dead 2\left4dead2\addons"
map_info

# Or run in-place (scans current directory for .vpk files):
./target/release/map_info
```

In debug builds (`cargo run`), the app automatically scans the default L4D2 addons directory. In release builds, it scans the current working directory.

## Keybindings

| Key | Action |
|---|---|
| `q` | Exit |
| `s` | Open filename filter (fuzzy search) |
| `c` | Open coop map name filter (fuzzy search) |
| `d` | Show duplicate detection dialog |
| `f` | Show map codes for selected file |
| `t` | Toggle sort field (creation / modification time) |
| `r` | Toggle sort order (ascending / descending) |
| `Ctrl+X` | Clear current filter |
| `↑` / `↓` | Navigate file list |
| `→` | View selected file's mission content |
| `←` | Return to file list |
| `Ctrl+←` / `Ctrl+→` | Resize list/content split |
| `PgUp` / `PgDn` | Scroll content by page |
| `Home` / `End` | Jump to top / bottom |
| `Enter` / `Esc` | Dismiss dialogs |

## Project Structure

```
map_info/
  cli/
    Cargo.toml          # Crate manifest
    src/
      main.rs           # Entry point
      lib.rs            # Module declarations, VPK file discovery
      app.rs            # App struct, event loop, draw layout
      vpk.rs            # VPK v1 parser
      mission.rs        # Valve KeyValues parser
      app/
        screen.rs       # Screen / FilterMode enums
        event_handler.rs # Keyboard input handling
        list_layout.rs  # File list rendering + fuzzy matching
        content_layout.rs # Right-pane rendering
        list_state.rs   # File list scroll state
        content_state.rs # Content scroll state
        filter_state.rs # Filter input state
        sort_state.rs   # Sort field + direction state
```

## Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | TUI framework |
| `crossterm` | Terminal backend |
| `color-eyre` | Error reporting |
| `indexmap` | Order-preserving map for file list |
| `strsim` | String similarity (Jaro-Winkler, Levenshtein) |
| `textwrap` | Word wrapping |
| `arboard` | Clipboard access |
| `rayon` | Parallel iteration for background scanning |

## License

No license declared yet.

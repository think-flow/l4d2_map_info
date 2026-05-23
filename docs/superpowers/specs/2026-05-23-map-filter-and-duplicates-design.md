# Map Filter & Duplicate Detection

**Date:** 2026-05-23  
**Status:** approved

## Overview

Add two VPK file filtering features to the map_info TUI:

1. **`c` key — Map filter mode**: Filter files by matching against the `modes → coop → Map` value inside each VPK's mission content. Mutually exclusive with the existing `s` key filename filter.
2. **`d` key — Duplicate detection**: Scan all VPK files, detect duplicate `Map` values, and show a dialog listing files that share the same Map.

## Architecture

### New module: `cli/src/mission.rs`

Lightweight Valve KeyValues parser that extracts a single value:

- `pub fn parse_coop_map(raw: &str) -> Option<String>` — token-based state machine that tracks nesting depth and extracts the `Map` value under the `modes → coop` path
- Handles: quoted strings, `{}` nesting, `//` comments, tabs/spaces as whitespace
- Returns `None` if the path doesn't exist or format is invalid

### Modified: `cli/src/app.rs` (App struct)

New fields:

| Field | Type | Purpose |
|---|---|---|
| `show_map_filter` | `bool` | Whether map filter dialog is visible |
| `show_duplicates` | `bool` | Whether duplicate files dialog is visible |
| `map_cache` | `HashMap<String, String>` | Filename → map value cache |
| `duplicate_files` | `Vec<String>` | Files with duplicate map values |

`show_map_filter` and `show_filter_dialog` are mutually exclusive — only one filter mode active at a time. `input_state` is shared between them.

### Modified: `cli/src/app/event_handler.rs`

New key bindings in `list_handler`:

- **`c`** — Enter map filter mode: clear `input_state`, set `show_map_filter = true`, `show_filter_dialog = false`, `display_content = false`. Lazily populate `map_cache` by reading mission content from all VPK files via `VPKInfo::new()` + `get_mission()` + `parse_coop_map()`.
- **`d`** — Show duplicates dialog: populate `map_cache` if not already done, detect duplicate map values, collect filenames into `duplicate_files`, set `show_duplicates = true`.
- `Enter`/`Esc` — Dismiss duplicates dialog (`show_duplicates = false`).

`s` key behavior: sets `show_map_filter = false`, `show_filter_dialog = true` (existing behavior, now also clears the other filter mode).

### Modified: `cli/src/app/list_layout.rs`

`match_file()` updated to accept a filter mode:
- **Filename mode**: existing fuzzy matching (Jaro-Winkler + Levenshtein)
- **Map mode**: substring matching against `map_cache` values
- **No filter**: return all files (existing behavior)

### Modified: `cli/src/app/content_layout.rs`

- Map filter dialog: same visual style as filename filter, but title shows "Map过滤器"
- Duplicates dialog: a scrollable popup listing duplicate filenames

### Data flow

```
Press 'c' → populate map_cache
  for each .vpk file:
    VPKInfo::new(path) → get_mission() → parse_coop_map(mission)
    → store in map_cache[filename] = map_value

Type in filter → match_file() in Map mode
  → filter map_cache by substring match → render filtered list

Press 'd' → populate map_cache (if needed)
  → group files by map_value
  → collect files where map_value appears > 1 time
  → show in duplicates dialog
```

### Error handling

- If a VPK file fails to open or has no mission content, skip it (don't add to cache). The file simply won't appear in map-filtered results.
- If `parse_coop_map` returns `None`, skip that file for map filtering.

## Scope

This spec covers the map filter and duplicate detection features only. SQLite persistence of map values is out of scope (future enhancement). The existing filename filter behavior must remain unchanged.

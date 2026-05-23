# Map Filter & Duplicate Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `c` key map filter mode and `d` key duplicate detection to the map_info TUI.

**Architecture:** New `mission.rs` module parses Valve KeyValues format to extract `modes → coop → Map`. App gets new state fields (`show_map_filter`, `show_duplicates`, `map_cache`, `duplicate_files`). Event handler dispatches `c`/`d` keys before list/content handlers. List filtering branches on filter mode (filename vs map). Content rendering shows mode-appropriate dialog titles and a duplicates popup.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.29, indexmap

---

### Task 1: Create mission.rs parser

**Files:**
- Create: `cli/src/mission.rs`

- [ ] **Step 1: Write the parser with tokenizer and path extraction**

```rust
/// Tokenize Valve KeyValues text into a flat list of tokens.
/// Handles quoted strings, braces, and // comments.
#[derive(Debug, PartialEq)]
enum Token {
    String(String),
    OpenBrace,
    CloseBrace,
}

fn tokenize(raw: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = raw.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\n' {
                            break;
                        }
                    }
                }
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '"' {
                        break;
                    }
                    s.push(c);
                }
                tokens.push(Token::String(s));
            }
            '{' => {
                chars.next();
                tokens.push(Token::OpenBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::CloseBrace);
            }
            _ if ch.is_whitespace() => {
                chars.next();
            }
            _ => {
                // Skip unexpected characters
                chars.next();
            }
        }
    }

    tokens
}

fn expect_brace(tokens: &[Token], pos: &mut usize, open: bool) -> bool {
    let expected = if open { Token::OpenBrace } else { Token::CloseBrace };
    if *pos < tokens.len() && tokens[*pos] == expected {
        *pos += 1;
        true
    } else {
        false
    }
}

/// Extract the Map value under modes → coop from Valve KeyValues text.
/// Returns None if the path doesn't exist or the format is invalid.
pub fn parse_coop_map(raw: &str) -> Option<String> {
    let tokens = tokenize(raw);
    let mut pos = 0;

    // Find "modes"
    while pos < tokens.len() {
        if let Token::String(s) = &tokens[pos] {
            if s == "modes" {
                pos += 1;
                break;
            }
        }
        pos += 1;
    }
    if pos >= tokens.len() {
        return None;
    }

    // Expect '{'
    if !expect_brace(&tokens, &mut pos, true) {
        return None;
    }

    // Search inside modes block for "coop"
    let mut depth: u32 = 1;
    while depth > 0 && pos < tokens.len() {
        match &tokens[pos] {
            Token::OpenBrace => depth += 1,
            Token::CloseBrace => depth -= 1,
            Token::String(s) if s == "coop" && depth == 1 => {
                pos += 1;
                if !expect_brace(&tokens, &mut pos, true) {
                    return None;
                }

                // Search inside coop block for "Map"
                let mut inner_depth: u32 = 1;
                while inner_depth > 0 && pos < tokens.len() {
                    match &tokens[pos] {
                        Token::OpenBrace => inner_depth += 1,
                        Token::CloseBrace => inner_depth -= 1,
                        Token::String(s) if s == "Map" && inner_depth == 1 => {
                            pos += 1;
                            if pos < tokens.len() {
                                if let Token::String(val) = &tokens[pos] {
                                    return Some(val.clone());
                                }
                            }
                            return None;
                        }
                        _ => {}
                    }
                    pos += 1;
                }
                return None;
            }
            _ => {}
        }
        pos += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coop_map_basic() {
        let input = r#"
"modes"
{
    "coop"
    {
        "Map" "c1m1_hotel"
    }
}
"#;
        assert_eq!(parse_coop_map(input), Some("c1m1_hotel".into()));
    }

    #[test]
    fn parse_coop_map_with_comments() {
        let input = r#"
// Header comment
"modes"
{
    "coop"  // cooperative mode
    {
        "Map" "c2m1_highway"  // first map
    }
}
"#;
        assert_eq!(parse_coop_map(input), Some("c2m1_highway".into()));
    }

    #[test]
    fn parse_coop_map_sibling_keys_ignored() {
        let input = r#"
"modes"
{
    "versus"
    {
        "Map" "c5m1_waterfront"
    }
    "coop"
    {
        "Map" "c1m1_hotel"
    }
}
"#;
        assert_eq!(parse_coop_map(input), Some("c1m1_hotel".into()));
    }

    #[test]
    fn parse_coop_map_missing() {
        let input = r#"
"modes"
{
    "versus"
    {
        "Map" "c5m1_waterfront"
    }
}
"#;
        assert_eq!(parse_coop_map(input), None);
    }

    #[test]
    fn parse_coop_map_empty() {
        assert_eq!(parse_coop_map(""), None);
    }

    #[test]
    fn parse_coop_map_no_modes() {
        assert_eq!(parse_coop_map(r#""something" { "key" "val" }"#), None);
    }

    #[test]
    fn parse_coop_map_tabs_instead_of_spaces() {
        let input = "\"modes\"\n{\n\t\"coop\"\n\t{\n\t\t\"Map\"\t\"c1m1_hotel\"\n\t}\n}";
        assert_eq!(parse_coop_map(input), Some("c1m1_hotel".into()));
    }
}
```

- [ ] **Step 2: Run tests to verify parser works**

```bash
cd cli && cargo test mission
```

Expected: All 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add cli/src/mission.rs
git commit -m "feat: add mission KeyValues parser for modes→coop→Map extraction"
```

---

### Task 2: Register mission module in lib.rs

**Files:**
- Modify: `cli/src/lib.rs`

- [ ] **Step 1: Add `pub mod mission;` declaration**

```rust
// In cli/src/lib.rs, add after the existing `mod vpk;` line:
pub mod app;
mod vpk;
pub mod mission;    // <-- add this line
```

- [ ] **Step 2: Verify it compiles**

```bash
cd cli && cargo check
```

Expected: Compiles without errors (new module is unused but valid).

- [ ] **Step 3: Commit**

```bash
git add cli/src/lib.rs
git commit -m "feat: register mission module in lib.rs"
```

---

### Task 3: Add App fields and cache/duplicate methods

**Files:**
- Modify: `cli/src/app.rs`

- [ ] **Step 1: Add new fields to App struct**

Replace the existing App struct definition:

```rust
#[derive(Debug, Default)]
pub struct App {
    vpk_files: indexmap::IndexMap<String, PathBuf>,
    selected_state: ListState,
    scroll_state: content_layout::ScrollState,
    selected_file_name: Option<String>,
    input_state: String,

    list_layout_percentage: u16,

    show_filter_dialog: bool,
    show_map_filter: bool,
    show_duplicates: bool,
    display_content: bool,

    map_cache: std::collections::HashMap<String, String>,
    duplicate_files: Vec<String>,
    duplicate_list_state: ListState,

    exit: bool,
}
```

- [ ] **Step 2: Add `populate_map_cache` method to `impl App`**

Add below the `init` method (after line ~47):

```rust
fn populate_map_cache(&mut self) -> color_eyre::Result<()> {
    if !self.map_cache.is_empty() {
        return Ok(());
    }
    for (file_name, file_path) in &self.vpk_files {
        if let Ok(vpk_info) = vpk::VPKInfo::new(file_path) {
            if let Ok(mission) = vpk_info.get_mission() {
                if let Some(map_value) = mission::parse_coop_map(&mission) {
                    self.map_cache
                        .insert(file_name.clone(), map_value);
                }
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Add `find_duplicates` method to `impl App`**

Add after `populate_map_cache`:

```rust
fn find_duplicates(&mut self) {
    self.duplicate_files.clear();
    let mut value_to_files: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (file_name, map_value) in &self.map_cache {
        value_to_files
            .entry(map_value.as_str())
            .or_default()
            .push(file_name.as_str());
    }
    for files in value_to_files.values() {
        if files.len() > 1 {
            for &f in files {
                self.duplicate_files.push(f.to_owned());
            }
        }
    }
    self.duplicate_files.sort();
    self.duplicate_list_state.select(Some(0));
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cd cli && cargo check
```

Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add cli/src/app.rs
git commit -m "feat: add map_cache, duplicate_files state and cache population methods"
```

---

### Task 4: Update event handler for c and d keys

**Files:**
- Modify: `cli/src/app/event_handler.rs`

- [ ] **Step 1: Add c/d key handling in `handle_events`**

Replace the `handle_events` method. New code goes after the Ctrl+Left/Right resize block and before the `list_handler` call:

```rust
pub(super) fn handle_events(&mut self) -> color_eyre::Result<()> {
    if let crossterm::event::Event::Key(key_event) = crossterm::event::read()?
        && key_event.is_press()
    {
        // Ctrl+C
        if key_event.code == KeyCode::Char('c')
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.exit = true;
            return Ok(());
        }

        // ctrl+<- 缩小list比例  ctrl+-> 放大list比例
        if (key_event.code == KeyCode::Left || key_event.code == KeyCode::Right)
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            match key_event.code {
                KeyCode::Left => {
                    if self.list_layout_percentage > 10 {
                        self.list_layout_percentage -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.list_layout_percentage < 80 {
                        self.list_layout_percentage += 1;
                    }
                }
                _ => unreachable!(),
            }
            return Ok(());
        }

        // Duplicates dialog dismiss (must come before c/d handlers)
        if self.show_duplicates {
            match key_event.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.show_duplicates = false;
                }
                KeyCode::Up => self.duplicate_list_state.select_previous(),
                KeyCode::Down => self.duplicate_list_state.select_next(),
                _ => {}
            }
            return Ok(());
        }

        // c key — enter map filter mode (skip if already in map filter, so 'c' can be typed)
        if key_event.code == KeyCode::Char('c') && !self.show_map_filter {
            self.show_filter_dialog = false;
            self.show_map_filter = true;
            self.display_content = false;
            self.input_state.clear();
            self.selected_state.select(Some(0));
            self.populate_map_cache()?;
            return Ok(());
        }

        // d key — show duplicates dialog
        if key_event.code == KeyCode::Char('d') {
            self.populate_map_cache()?;
            self.find_duplicates();
            self.show_duplicates = true;
            self.show_filter_dialog = false;
            self.show_map_filter = false;
            return Ok(());
        }

        if list_handler(self, key_event)?.is_some() {
            return Ok(());
        }

        if content_handler(self, key_event)?.is_some() {
            return Ok(());
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Add s key handling in list_handler to clear map filter when entering filename filter**

In `list_handler`, modify the `KeyCode::Char('s')` branch to also clear `show_map_filter`:

Find the line `KeyCode::Char('s') => app.show_filter_dialog = true,` and replace with:

```rust
KeyCode::Char('s') => {
    app.show_filter_dialog = true;
    app.show_map_filter = false;
    app.input_state.clear();
    app.selected_state.select(Some(0));
}
```

- [ ] **Step 3: Also clear map filter on Ctrl+X**

In `list_handler`, find the Ctrl+X handler block and update it:

```rust
// Ctrl+X 清空文件过滤器
if key_event.code == KeyCode::Char('x') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
    app.input_state.clear();
    app.show_map_filter = false;
    app.selected_state.select(Some(0));
    return PREVENT;
}
```

- [ ] **Step 4: Handle map filter input in list_handler's filter dialog branch**

In `list_handler`, change the condition `if app.show_filter_dialog` to `if app.show_filter_dialog || app.show_map_filter`:

Find:
```rust
if app.show_filter_dialog {
```

Replace with:
```rust
if app.show_filter_dialog || app.show_map_filter {
```

And in that branch, change the `Enter`/`Esc` handling to also clear `show_map_filter`:

Find:
```rust
KeyCode::Enter | KeyCode::Esc | KeyCode::Up | KeyCode::Down => {
    app.show_filter_dialog = false;
```

Replace with:
```rust
KeyCode::Enter | KeyCode::Esc | KeyCode::Up | KeyCode::Down => {
    app.show_filter_dialog = false;
    app.show_map_filter = false;
```

- [ ] **Step 5: Verify it compiles**

```bash
cd cli && cargo check
```

Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add cli/src/app/event_handler.rs
git commit -m "feat: add c key map filter and d key duplicates event handling"
```

---

### Task 5: Update list filtering for map mode

**Files:**
- Modify: `cli/src/app/list_layout.rs`

- [ ] **Step 1: Update `render_list` to pass filter mode to `match_file`**

In `render_list`, replace the `match_file` call. Find:

```rust
let matched_list = match_file(&self.input_state, &self.vpk_files);
```

Replace with:

```rust
let matched_list = if self.show_map_filter {
    match_file_by_map(&self.input_state, &self.vpk_files, &self.map_cache)
} else {
    match_file(&self.input_state, &self.vpk_files)
};
```

- [ ] **Step 2: Add `match_file_by_map` function after the existing `match_file` function**

Add at the end of the file:

```rust
/// Filter vpk_files by matching input against cached map values (substring match).
/// Returns an empty vec if no matches.
fn match_file_by_map<'a, 'b>(
    input: &'a str,
    vpk_files: &'b indexmap::IndexMap<String, PathBuf>,
    map_cache: &'b std::collections::HashMap<String, String>,
) -> Vec<&'b str> {
    if input.is_empty() {
        return vpk_files.keys().map(|v| v.as_str()).collect();
    }
    let input_lower = input.to_lowercase();
    vpk_files
        .keys()
        .filter(|file_name| {
            if let Some(map_value) = map_cache.get(*file_name) {
                map_value.to_lowercase().contains(&input_lower)
            } else {
                false
            }
        })
        .map(|v| v.as_str())
        .collect()
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd cli && cargo check
```

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add cli/src/app/list_layout.rs
git commit -m "feat: add map value filtering to file list"
```

---

### Task 6: Update content rendering for filter dialog titles and duplicates dialog

**Files:**
- Modify: `cli/src/app/content_layout.rs`

- [ ] **Step 1: Update `render_filter_dialog` to show correct title for map filter mode**

Find the `render_filter_dialog` function. Replace the title line:

```rust
let popup_block = Block::bordered()
    .title("文件过滤器")
    .border_type(BorderType::Double);
```

Replace with:

```rust
let title = if app.show_map_filter { "Map过滤器" } else { "文件过滤器" };
let popup_block = Block::bordered()
    .title(title)
    .border_type(BorderType::Double);
```

- [ ] **Step 2: Update the `render_filter_dialog` call site to handle both filter modes**

In the `impl App` block's `render_content` method, update the condition for showing the filter dialog. Find:

```rust
render_filter_dialog(self, frame, area);
```

Replace with:

```rust
if self.show_filter_dialog || self.show_map_filter {
    render_filter_dialog(self, frame, area);
}
if self.show_duplicates {
    render_duplicates_dialog(self, frame, area);
}
```

- [ ] **Step 3: Add `render_duplicates_dialog` function at the end of the file**

```rust
fn render_duplicates_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(format!(" 重复文件({}) ", app.duplicate_files.len()))
        .border_type(BorderType::Double);

    let items: Vec<ListItem> = app
        .duplicate_files
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().reversed());

    let dialog_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(60));
    frame.render_widget(Clear, dialog_area);
    frame.render_stateful_widget(list, dialog_area, &mut app.duplicate_list_state);
}
```

- [ ] **Step 4: Make sure all needed imports are available**

Check that `Clear`, `List`, `ListItem`, `Style` are imported in `content_layout.rs`. They should already be available via `use super::*` and the imports in `app.rs`. The `Constraint` import comes from `ratatui::layout::*`. All needed types are already imported.

- [ ] **Step 5: Verify it compiles**

```bash
cd cli && cargo check
```

Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add cli/src/app/content_layout.rs
git commit -m "feat: add map filter dialog title and duplicates dialog"
```

---

### Final verification

- [ ] **Run full check**

```bash
cd cli && cargo check && cargo test
```

Expected: All tests pass, no warnings.

- [ ] **Run the app and manually test**

```bash
cd cli && cargo run
```

Test scenarios:
1. Press `s` → file filter dialog appears with "文件过滤器" title
2. Press `c` → map filter dialog appears with "Map过滤器" title, type to filter by map name
3. Press `d` → duplicates dialog appears listing files with duplicate map values
4. `Enter`/`Esc` dismisses dialogs
5. `Ctrl+X` clears filter
6. `s` and `c` are mutually exclusive (switching clears the other)
7. Ensure existing file list navigation still works

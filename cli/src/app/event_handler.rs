use crate::app::App;
use crate::app::screen::{FilterMode, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

type HandlerResult = color_eyre::Result<()>;

pub fn handle_events(app: &mut App) -> HandlerResult {
    if !crossterm::event::poll(std::time::Duration::from_millis(100))? {
        return Ok(());
    }
    if let crossterm::event::Event::Key(key_event) = crossterm::event::read()?
        && key_event.is_press()
    {
        // Global: Ctrl+C
        if key_event.code == KeyCode::Char('c')
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            app.exit = true;
            return Ok(());
        }

        // Global: Ctrl+arrow resize list/content split
        if (key_event.code == KeyCode::Left || key_event.code == KeyCode::Right)
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            match key_event.code {
                KeyCode::Left if app.list_layout_percentage > 10 => {
                    app.list_layout_percentage -= 1;
                }
                KeyCode::Right if app.list_layout_percentage < 80 => {
                    app.list_layout_percentage += 1;
                }
                _ => {}
            }
            return Ok(());
        }

        // Global: Ctrl+X clear filter and reset to filename mode
        if key_event.code == KeyCode::Char('x')
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            app.filter.input.clear();
            app.filter.mode = FilterMode::FileName;
            app.show_toast("已清除过滤器");
            return Ok(());
        }

        // Dispatch by screen
        match app.screen {
            Screen::Content => content_handler(app, key_event)?,
            Screen::Filter(_) => filter_handler(app, key_event)?,
            Screen::Duplicates => duplicates_handler(app, key_event)?,
            Screen::MapCodes => map_codes_handler(app, key_event)?,
            Screen::FileList => list_handler(app, key_event)?,
        }
    }
    Ok(())
}

fn list_handler(app: &mut App, key: KeyEvent) -> HandlerResult {
    match key.code {
        KeyCode::Up => app.list.select_previous(),
        KeyCode::Down => app.list.select_next(),
        KeyCode::End => app.list.select_last(),
        KeyCode::Home => app.list.select_first(),
        KeyCode::PageUp => app.list.page_up(),
        KeyCode::PageDown => app.list.page_down(),
        KeyCode::Char('s') => {
            app.screen = Screen::Filter(FilterMode::FileName);
            app.filter.mode = FilterMode::FileName;
            app.filter.input.clear();
            app.list.select(Some(0));
        }
        KeyCode::Char('c') => {
            app.screen = Screen::Filter(FilterMode::MapName);
            app.filter.mode = FilterMode::MapName;
            app.filter.input.clear();
            app.list.select(Some(0));
        }
        KeyCode::Char('d') => {
            app.find_duplicates();
            app.screen = Screen::Duplicates;
        }
        KeyCode::Char('t') if app.filter.input.is_empty() => {
            app.sort.toggle_field();
            app.sort
                .apply_to(&mut app.vpk_files, &app.file_created, &app.file_modified);
            app.list.select(Some(0));
        }
        KeyCode::Char('r') if app.filter.input.is_empty() => {
            app.sort.toggle_order();
            app.sort
                .apply_to(&mut app.vpk_files, &app.file_created, &app.file_modified);
            app.list.select(Some(0));
        }
        KeyCode::Char('q') => app.exit = true,
        KeyCode::Char('f') => {
            if app.selected_file_name.is_some() {
                app.map_codes_list.select(Some(0));
                app.screen = Screen::MapCodes;
            }
        }
        KeyCode::Right => {
            if app.selected_file_name.is_some() {
                app.screen = Screen::Content;
            }
        }
        _ => {}
    }
    Ok(())
}

fn content_handler(app: &mut App, key: KeyEvent) -> HandlerResult {
    match key.code {
        KeyCode::Char('q') => app.exit = true,
        KeyCode::Left => {
            app.screen = Screen::FileList;
            app.content.reset();
        }
        KeyCode::Up => app.content.scroll_up(),
        KeyCode::Down => app.content.scroll_down(),
        KeyCode::End => app.content.scroll_to_bottom(),
        KeyCode::Home => app.content.scroll_to_top(),
        KeyCode::PageUp => app.content.scroll_page_up(),
        KeyCode::PageDown => app.content.scroll_page_down(),
        _ => {}
    }
    Ok(())
}

fn filter_handler(app: &mut App, key: KeyEvent) -> HandlerResult {
    match key.code {
        KeyCode::Char(c) => {
            app.filter.input.push(c);
            app.list.select(Some(0));
        }
        KeyCode::Backspace => {
            app.filter.input.pop();
            app.list.select(Some(0));
        }
        KeyCode::Enter | KeyCode::Esc => {
            app.screen = Screen::FileList;
        }
        KeyCode::Up => {
            app.screen = Screen::FileList;
            app.list.select_previous();
        }
        KeyCode::Down => {
            app.screen = Screen::FileList;
            app.list.select_next();
        }
        _ => {}
    }
    Ok(())
}

fn duplicates_handler(app: &mut App, key: KeyEvent) -> HandlerResult {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            app.screen = Screen::FileList;
        }
        KeyCode::Up => app.duplicate_list.select_previous(),
        KeyCode::Down => app.duplicate_list.select_next(),
        _ => {}
    }
    Ok(())
}

fn map_codes_handler(app: &mut App, key: KeyEvent) -> HandlerResult {
    match key.code {
        KeyCode::Up => app.map_codes_list.select_previous(),
        KeyCode::Down => app.map_codes_list.select_next(),
        KeyCode::Enter => {
            if let Some(idx) = app.map_codes_list.selected() {
                if let Some(name) = app.selected_file_name.as_ref()
                    && let Some(maps) = app.map_cache.get(name.as_str())
                    && let Some(code) = maps.get(idx)
                {
                    if arboard::Clipboard::new()
                        .and_then(|mut cb| cb.set_text(code.clone()))
                        .is_ok()
                    {
                        app.show_toast(format!("已复制地图代码: {}", code));
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.screen = Screen::FileList;
        }
        _ => {}
    }
    Ok(())
}

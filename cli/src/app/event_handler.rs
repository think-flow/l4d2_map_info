use super::*;

type HandlerResult = color_eyre::Result<Option<()>>;
// 阻止事件继续传播
const PREVENT: HandlerResult = Ok(Some(()));
// 允许事件继续传播
const ALLOW: HandlerResult = Ok(None);

impl App {
    // 检测键盘事件
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

            // c key — enter map filter mode (not when viewing content or any dialog open)
            if key_event.code == KeyCode::Char('c')
                && !self.display_content
                && !self.show_filter_dialog
                && !self.show_map_dialog
                && !self.show_duplicates
            {
                self.show_filter_dialog = false;
                self.show_map_dialog = true;
                self.use_map_filter = true;
                self.display_content = false;
                self.input_state.clear();
                self.selected_state.select(Some(0));
                self.populate_map_cache()?;
                return Ok(());
            }

            // d key — show duplicates dialog (not when viewing content or any dialog open)
            if key_event.code == KeyCode::Char('d')
                && !self.display_content
                && !self.show_filter_dialog
                && !self.show_map_dialog
                && !self.show_duplicates
            {
                self.populate_map_cache()?;
                self.find_duplicates();
                self.show_duplicates = true;
                self.show_filter_dialog = false;
                self.show_map_dialog = false;
                self.use_map_filter = false;
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
}

fn list_handler(app: &mut App, key_event: KeyEvent) -> HandlerResult {
    if app.display_content {
        return ALLOW;
    }

    // Ctrl+X 清空文件过滤器
    if key_event.code == KeyCode::Char('x') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
        app.input_state.clear();
        app.use_map_filter = false;
        app.show_map_dialog = false;
        app.selected_state.select(Some(0));
        return PREVENT;
    }

    if app.show_filter_dialog || app.show_map_dialog {
        // 显示了文件过滤器 分支
        match key_event.code {
            KeyCode::Char(c) => {
                app.input_state.push(c);
                app.selected_state.select(Some(0));
            }
            KeyCode::Backspace => {
                app.input_state.pop();
                app.selected_state.select(Some(0));
            }
            KeyCode::Enter | KeyCode::Esc | KeyCode::Up | KeyCode::Down => {
                app.show_filter_dialog = false;
                app.show_map_dialog = false;
                match key_event.code {
                    KeyCode::Up => {
                        app.selected_state.select_previous();
                    }
                    KeyCode::Down => {
                        app.selected_state.select_next();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    } else {
        // 未显示文件过滤器 分支
        match key_event.code {
            KeyCode::Up => app.selected_state.select_previous(),
            KeyCode::Down => app.selected_state.select_next(),
            KeyCode::End => app.selected_state.select_last(),
            KeyCode::Home => app.selected_state.select_first(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            KeyCode::Char('s') => {
                app.show_filter_dialog = true;
                app.use_map_filter = false;
                app.show_map_dialog = false;
                app.input_state.clear();
                app.selected_state.select(Some(0));
            }
            KeyCode::Char('t') if app.input_state.is_empty() => {
                app.sort_by_created = !app.sort_by_created;
                app.sort_files();
                app.selected_state.select(Some(0));
            }
            KeyCode::Char('r') if app.input_state.is_empty() => {
                app.sort_ascending = !app.sort_ascending;
                app.sort_files();
                app.selected_state.select(Some(0));
            }
            KeyCode::Char('q') => app.exit = true,
            KeyCode::Right => {
                if app.selected_file_name.is_some() {
                    app.display_content = true;
                }
            }
            _ => {}
        }
    }

    ALLOW
}

fn content_handler(app: &mut App, key_event: KeyEvent) -> HandlerResult {
    if !app.display_content {
        return ALLOW;
    }

    match key_event.code {
        KeyCode::Char('q') => app.exit = true,
        KeyCode::Left => {
            app.display_content = false;
            app.scroll_state.reset();
        }
        KeyCode::Up => app.scroll_state.scroll_up(),
        KeyCode::Down => app.scroll_state.scroll_down(),
        KeyCode::End => app.scroll_state.scroll_to_bottom(),
        KeyCode::Home => app.scroll_state.scroll_to_top(),
        KeyCode::PageUp => app.scroll_state.scroll_page_up(),
        KeyCode::PageDown => app.scroll_state.scroll_page_down(),
        _ => {}
    }

    ALLOW
}

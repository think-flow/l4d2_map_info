use super::*;

#[derive(Debug, Default)]
pub(super) struct ScrollState {
    scroll_offset: u16,
    line_count: u16,
    area_height: u16,
}

impl App {
    pub(super) fn render_content(
        &mut self,
        frame: &mut Frame,
        area: Rect,
    ) -> color_eyre::Result<()> {
        let block = Block::bordered()
            .title(" mission内容 ")
            .title_alignment(HorizontalAlignment::Center);
        frame.render_widget(block, area);

        render_content(self, frame, area)?;
        if self.show_filter_dialog || self.show_map_dialog {
            render_filter_dialog(self, frame, area);
        }
        if self.show_duplicates {
            render_duplicates_dialog(self, frame, area);
        }
        Ok(())
    }
}

fn render_content(app: &mut App, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
    if !app.display_content {
        render_select_file_hit(frame, area);
        return Ok(());
    }

    if let Some(file_name) = &app.selected_file_name
        && let Some(file_path) = app.vpk_files.get(file_name)
    {
        match vpk::VPKInfo::new(file_path) {
            Ok(vpk_info) => match vpk_info.get_mission() {
                Ok(mission) => {
                    // 这里原始文件内容使用 tab 制表符，
                    // 所以我们需要先将 tab 转为空格，这样Paragraph才能正确显示缩进
                    let mission = mission.replace('\t', "    ");
                    // 创建bolock
                    let mut block = Block::bordered()
                        .border_type(BorderType::Double)
                        .title_alignment(HorizontalAlignment::Center);
                    let inner_area = block.inner(area);

                    let mut paragraph = Paragraph::new(mission).wrap(Wrap { trim: false });
                    // 获取行数
                    let line_count: u16 = paragraph
                        .line_count(inner_area.width)
                        .try_into()
                        .map_err(|err| {
                            color_eyre::Report::new(err).wrap_err("文件内容不能超过65535行")
                        })?;
                    app.scroll_state.area_height = inner_area.height;
                    app.scroll_state.line_count = line_count;
                    paragraph = paragraph.scroll((app.scroll_state.scroll_offset, 0));

                    block = block.title(format!(
                        " mission内容 {}% ",
                        if line_count == 0 || app.scroll_state.get_viewd_offset() >= line_count {
                            100
                        } else {
                            app.scroll_state.get_viewd_offset() * 100 / line_count
                        }
                    ));

                    frame.render_widget(block, area);
                    frame.render_widget(paragraph, inner_area);
                }
                Err(e) => {
                    render_error_hit(frame, area, e);
                }
            },
            Err(e) => {
                render_error_hit(frame, area, e);
            }
        }
    } else {
        // 没有选中文件
        render_select_file_hit(frame, area);
    }

    Ok(())
}

fn render_select_file_hit(frame: &mut Frame, area: Rect) {
    let centered_area = area.centered(Constraint::Percentage(50), Constraint::Length(1));
    frame.render_widget(Paragraph::new("请选择文件").centered(), centered_area);
}

fn render_error_hit(frame: &mut Frame, area: Rect, e: color_eyre::Report) {
    let err_str = format!("{}", e);
    let centered_area = area.centered(Constraint::Percentage(50), Constraint::Length(2));
    frame.render_widget(Paragraph::new(err_str).centered(), centered_area);
}

fn render_filter_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    if !app.show_filter_dialog && !app.show_map_dialog {
        return;
    }

    // 弹出对话框
    let title = if app.use_map_filter {
        " Map过滤器 ".to_owned()
    } else {
        " 文件过滤器 ".to_owned()
    };
    let popup_block = Block::bordered()
        .title(title)
        .border_type(BorderType::Double);

    let line = Line::from(vec![Span::raw(&app.input_state), Span::raw("█")]);
    let paragraph = Paragraph::new(line);

    // 第一设置弹出框宽度， 第二个设置弹出框高度
    let centered_area = area.centered(Constraint::Percentage(50), Constraint::Length(3));
    frame.render_widget(Clear, centered_area);
    frame.render_widget(paragraph.block(popup_block), centered_area);
}

fn render_duplicates_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    let group_count = app.duplicate_groups.len();
    let block = Block::bordered()
        .title(format!(" 重复文件({}) ", group_count))
        .border_type(BorderType::Double);

    let mut items: Vec<ListItem> = Vec::new();
    for (i, files) in app.duplicate_groups.iter().enumerate() {
        if i > 0 {
            items.push(ListItem::new(""));
        }
        items.push(ListItem::new(format!("组 {}:", i + 1)));
        for file in files {
            items.push(ListItem::new(format!("  - {}", file)));
        }
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().reversed());

    let dialog_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(60));
    frame.render_widget(Clear, dialog_area);
    frame.render_stateful_widget(list, dialog_area, &mut app.duplicate_list_state);
}

impl ScrollState {
    /// 重置状态
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 向上滚动一行
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// 向下滚动一行
    pub fn scroll_down(&mut self) {
        if self.get_viewd_offset() <= self.line_count {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    /// 滚动到最底端
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.get_botton_offset();
    }

    /// 滚动到最顶端
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// 滚动到上一页
    pub fn scroll_page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(self.area_height);
    }

    /// 滚动到下一页
    pub fn scroll_page_down(&mut self) {
        let result = self.scroll_offset.saturating_add(self.area_height);
        let bottom_offset = self.get_botton_offset();
        if result > bottom_offset {
            self.scroll_offset = bottom_offset;
        } else {
            self.scroll_offset = result;
        }
    }

    // 获取最底部的scroll_offset的偏移量
    fn get_botton_offset(&self) -> u16 {
        self.line_count - self.area_height + 1
    }

    fn get_viewd_offset(&self) -> u16 {
        // scroll_offset + area_height
        self.scroll_offset + self.area_height
    }
}

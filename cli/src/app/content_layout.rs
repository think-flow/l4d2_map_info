use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::*,
};

use crate::app::App;
use crate::app::filter_state::FilterState;

pub fn render_mission(app: &mut App, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
    // Always render outer border first
    let outer_block = Block::bordered()
        .title(" mission内容 ")
        .title_alignment(Alignment::Center);
    frame.render_widget(outer_block, area);

    if let Some(file_name) = &app.selected_file_name
        && let Some(file_path) = app.vpk_files.get(file_name)
    {
        match crate::vpk::VPKInfo::new(file_path) {
            Ok(vpk_info) => match vpk_info.get_mission() {
                Ok(mission) => {
                    let mission = mission.replace('\t', "    ");
                    let mut block = Block::bordered()
                        .border_type(BorderType::Double)
                        .title_alignment(Alignment::Center);
                    let inner_area = block.inner(area);

                    let mut paragraph = Paragraph::new(mission).wrap(Wrap { trim: false });
                    let line_count: u16 = paragraph
                        .line_count(inner_area.width)
                        .try_into()
                        .map_err(|err| {
                            color_eyre::Report::new(err).wrap_err("文件内容不能超过65535行")
                        })?;
                    app.content.area_height = inner_area.height;
                    app.content.line_count = line_count;
                    paragraph = paragraph.scroll((app.content.scroll_offset, 0));

                    block = block.title(format!(
                        " mission内容 {}% ",
                        if line_count == 0
                            || app.content.scroll_offset + app.content.area_height >= line_count
                        {
                            100
                        } else {
                            (app.content.scroll_offset + app.content.area_height) * 100 / line_count
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
        render_select_file_hit(frame, area);
    }

    Ok(())
}

pub fn render_hint(frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(" mission内容 ")
        .title_alignment(Alignment::Center);
    frame.render_widget(block, area);

    render_select_file_hit(frame, area);
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

pub fn render_filter_dialog(
    frame: &mut Frame,
    area: Rect,
    filter: &FilterState,
) -> color_eyre::Result<()> {
    let title = match filter.mode {
        crate::app::filter_state::FilterMode::MapName => " Map过滤器 ".to_owned(),
        crate::app::filter_state::FilterMode::FileName => " 文件过滤器 ".to_owned(),
    };
    let popup_block = Block::bordered()
        .title(title)
        .border_type(BorderType::Double);

    let line = Line::from(vec![Span::raw(&filter.input), Span::raw("█")]);
    let paragraph = Paragraph::new(line);

    let centered_area = area.centered(Constraint::Percentage(50), Constraint::Length(3));
    frame.render_widget(Clear, centered_area);
    frame.render_widget(paragraph.block(popup_block), centered_area);
    Ok(())
}

pub fn render_duplicates_dialog(
    frame: &mut Frame,
    area: Rect,
    duplicate_groups: &[Vec<String>],
    list_state: &mut ratatui::widgets::ListState,
) -> color_eyre::Result<()> {
    let group_count = duplicate_groups.len();
    let block = Block::bordered()
        .title(format!(" 重复文件({}) ", group_count))
        .border_type(BorderType::Double);

    let mut items: Vec<ListItem> = Vec::new();
    for (i, files) in duplicate_groups.iter().enumerate() {
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
    frame.render_stateful_widget(list, dialog_area, list_state);
    Ok(())
}

pub fn render_map_codes_dialog(
    frame: &mut Frame,
    area: Rect,
    map_codes: Option<&[String]>,
    list_state: &mut ratatui::widgets::ListState,
) -> color_eyre::Result<()> {
    let (title, items): (String, Vec<ListItem>) = match map_codes {
        Some(codes) if !codes.is_empty() => {
            let items: Vec<ListItem> = codes
                .iter()
                .enumerate()
                .map(|(i, code)| ListItem::new(format!(" {} - {} ", i + 1, code)))
                .collect();
            (format!(" 地图代码({}) ", codes.len()), items)
        }
        _ => (
            " 地图代码 ".to_owned(),
            vec![ListItem::new(" 该文件没有地图代码或尚未扫描 ")],
        ),
    };

    let block = Block::bordered()
        .title(title)
        .border_type(BorderType::Double);

    let has_items = map_codes.is_some_and(|c| !c.is_empty());
    let item_count = items.len();
    let dialog_area = area.centered(
        Constraint::Percentage(60),
        Constraint::Length((item_count as u16 + 2).min(20)),
    );
    frame.render_widget(Clear, dialog_area);
    if has_items {
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed());
        frame.render_stateful_widget(list, dialog_area, list_state);
    } else {
        frame.render_widget(List::new(items).block(block), dialog_area);
    }
    Ok(())
}

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::*,
};
use strsim::{jaro_winkler, levenshtein};

use crate::app::App;
use crate::app::screen::FilterMode;

pub fn render_list(app: &mut App, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
    if app.vpk_files.is_empty() {
        render_no_file_hit(frame, area);
        return Ok(());
    }

    let max_width = area.width.saturating_sub(3) as usize;
    let matched_list = match app.filter.mode {
        FilterMode::MapName => match_file_by_map(&app.filter.input, &app.vpk_files, &app.map_cache),
        FilterMode::FileName => match_file(&app.filter.input, &app.vpk_files),
    };

    // Compute wrapped text + line counts for page-aware scrolling
    let mut line_counts = Vec::with_capacity(matched_list.len());
    let wrapped_items: Vec<ListItem> = matched_list
        .iter()
        .map(|s| {
            if matches!(app.filter.mode, FilterMode::FileName) && !app.filter.input.is_empty() {
                let wrapped = textwrap::fill(s, max_width);
                let lines: Vec<Line> = wrapped
                    .lines()
                    .map(|line| Line::from(highlight_matches(line, &app.filter.input)))
                    .collect();
                line_counts.push(lines.len() as u16);
                ListItem::new(Text::from(lines))
            } else {
                let wrapped = textwrap::fill(s, max_width);
                line_counts.push(wrapped.lines().count() as u16);
                ListItem::new(wrapped)
            }
        })
        .collect();
    app.list.line_counts = line_counts;
    app.list.area_height = area.height.saturating_sub(2); // minus border
    let list_len = wrapped_items.len();

    let title = if app.filter.input.is_empty() {
        let time_label = if app.sort.field == crate::app::sort_state::SortField::CreatedAt {
            "创建"
        } else {
            "修改"
        };
        let order_label = if app.sort.ascending { "↑" } else { "↓" };
        format!(
            " 文件({}/{})  {}{} ",
            app.list
                .selected()
                .map_or(0, |v| { if v >= list_len { list_len } else { v + 1 } }),
            list_len,
            time_label,
            order_label,
        )
    } else {
        format!(
            " 文件({}/{}) ",
            app.list
                .selected()
                .map_or(0, |v| { if v >= list_len { list_len } else { v + 1 } }),
            list_len,
        )
    };

    let display_content = matches!(app.screen, crate::app::screen::Screen::Content);
    let mut block = Block::bordered()
        .title(title)
        .title_alignment(Alignment::Center);
    if !display_content {
        block = block.border_type(BorderType::Double);
    }
    let list = List::new(wrapped_items)
        .highlight_style(Style::new().reversed())
        .block(block)
        .highlight_symbol(">")
        .highlight_spacing(HighlightSpacing::Always)
        .repeat_highlight_symbol(false);

    frame.render_stateful_widget(list, area, &mut app.list.selected);

    // Update selected file name
    match app.list.selected() {
        Some(index) => {
            app.selected_file_name = Some(matched_list[index].to_owned());
        }
        None => app.selected_file_name = None,
    }

    Ok(())
}

fn render_no_file_hit(frame: &mut Frame, area: Rect) {
    let centered_area = area.centered(Constraint::Length(15), Constraint::Length(1));
    let paragraph = Paragraph::new("当前路径无文件！").wrap(Wrap { trim: false });
    frame.render_widget(paragraph.centered(), centered_area);
}

/// Highlight all occurrences of `input` in `text` (case-insensitive) with yellow foreground.
fn highlight_matches(text: &str, input: &str) -> Vec<Span<'static>> {
    if input.is_empty() {
        return vec![Span::raw(text.to_owned())];
    }
    let lower_text = text.to_lowercase();
    let lower_input = input.to_lowercase();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut start = 0;
    while let Some(pos) = lower_text[start..].find(&lower_input) {
        let abs_pos = start + pos;
        if abs_pos > start {
            spans.push(Span::raw(text[start..abs_pos].to_owned()));
        }
        spans.push(Span::styled(
            text[abs_pos..abs_pos + input.len()].to_owned(),
            Style::default().fg(Color::Yellow),
        ));
        start = abs_pos + input.len();
    }
    if start < text.len() {
        spans.push(Span::raw(text[start..].to_owned()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_owned()));
    }
    spans
}

/// input为空，则返回全部文件，如果没有匹配项则vec len为0
fn match_file<'a, 'b>(
    input: &'a str,
    vpk_files: &'b indexmap::IndexMap<String, std::path::PathBuf>,
) -> Vec<&'b str> {
    if input.is_empty() {
        return vpk_files.keys().map(|v| v.as_str()).collect();
    }

    let threshold = dyn_threshold(input);

    let mut matched_list: Vec<(&str, f64)> = vpk_files
        .keys()
        .flat_map(|content| {
            let score = score_keyword(input, content);
            if score >= threshold {
                Some((content.as_str(), score))
            } else {
                None
            }
        })
        .collect();
    matched_list.sort_by(|a, b| b.1.total_cmp(&a.1));
    matched_list.into_iter().map(|v| v.0).collect()
}

/// 平滑阈值：1 字符 ~0.18，4 ~0.43，8 ~0.77，12+ → 0.8
fn dyn_threshold(keyword: &str) -> f64 {
    let input_len = keyword.chars().count();
    if input_len == 0 {
        return 0.0;
    }
    0.1 + (input_len as f64 / 12.0).min(0.7)
}

/// 给定关键字和文本，返回相似度分数（0.0 ~ 1.0）
///
/// 使用 Jaro-Winkler + Levenshtein 混合评分，子串命中额外加分，
/// 加分按命中长度占比加权，且匹配位置越靠前分数越高。
fn score_keyword(keyword: &str, txt: &str) -> f64 {
    let keyword_lower = keyword.to_lowercase();
    let txt_lower = txt.to_lowercase();

    let jw_score = jaro_winkler(&txt_lower, &keyword_lower);
    let max_len = txt_lower.len().max(keyword_lower.len());
    let lev_dist = levenshtein(&txt_lower, &keyword_lower);
    let lev_score = 1.0 - (lev_dist as f64 / max_len as f64);
    let mut score = (jw_score + lev_score) / 2.0;

    // 子串命中加分：匹配越长、匹配越靠前，加分越多
    if let Some(pos) = txt_lower.find(&keyword_lower) {
        let match_ratio = keyword_lower.len() as f64 / txt_lower.len().max(1) as f64;
        let pos_factor = 1.0 - (pos as f64 / txt_lower.len().max(1) as f64) * 0.5;
        let bonus = 0.3 * (1.0 + match_ratio) * pos_factor;
        score = (score + bonus).min(1.0);
    }

    score
}

/// Filter vpk_files by matching input against cached map values (substring match).
fn match_file_by_map<'a, 'b>(
    input: &'a str,
    vpk_files: &'b indexmap::IndexMap<String, std::path::PathBuf>,
    map_cache: &'b std::collections::HashMap<String, Vec<String>>,
) -> Vec<&'b str> {
    if input.is_empty() {
        return vpk_files.keys().map(|v| v.as_str()).collect();
    }

    let threshold = dyn_threshold(input);

    let mut matched_list: Vec<(&str, f64)> = vpk_files
        .keys()
        .filter_map(|file_name| {
            let map_values = map_cache.get(file_name)?;
            let best_score = map_values
                .iter()
                .map(|v| score_keyword(input, v))
                .fold(0.0_f64, f64::max);
            if best_score >= threshold {
                Some((file_name.as_str(), best_score))
            } else {
                None
            }
        })
        .collect();
    matched_list.sort_by(|a, b| b.1.total_cmp(&a.1));
    matched_list.into_iter().map(|v| v.0).collect()
}

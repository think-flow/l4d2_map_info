use super::*;
use strsim::{jaro_winkler, levenshtein};

impl App {
    pub(super) fn render_list(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.vpk_files.is_empty() {
            render_no_file_hit(frame, area);
        }

        let max_width = area.width.saturating_sub(3) as usize;
        let matched_list = if self.use_map_filter {
            match_file_by_map(&self.input_state, &self.vpk_files, &self.map_cache)
        } else {
            match_file(&self.input_state, &self.vpk_files)
        };

        // Compute wrapped text + line counts for page-aware scrolling
        let mut line_counts = Vec::with_capacity(matched_list.len());
        let wrapped_items: Vec<ListItem> = matched_list
            .iter()
            .map(|s| {
                let wrapped = textwrap::fill(s, max_width);
                line_counts.push(wrapped.lines().count() as u16);
                ListItem::new(wrapped)
            })
            .collect();
        self.list_line_counts = line_counts;
        self.list_area_height = area.height.saturating_sub(2); // minus border
        let list_len = wrapped_items.len();

        let title = if self.input_state.is_empty() {
            let time_label = if self.sort_by_created { "创建" } else { "修改" };
            let order_label = if self.sort_ascending { "↑" } else { "↓" };
            format!(
                " 文件({}/{})  {}{} ",
                self.selected_state
                    .selected()
                    .map_or(0, |v| { if v >= list_len { list_len } else { v + 1 } }),
                list_len,
                time_label,
                order_label,
            )
        } else {
            format!(
                " 文件({}/{}) ",
                self.selected_state
                    .selected()
                    .map_or(0, |v| { if v >= list_len { list_len } else { v + 1 } }),
                list_len,
            )
        };
        let mut block = Block::bordered()
            .title(title)
            .title_alignment(HorizontalAlignment::Center);
        if !self.display_content {
            block = block.border_type(BorderType::Double);
        }
        let list = List::new(wrapped_items)
            .highlight_style(Style::new().reversed())
            .block(block)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always)
            .repeat_highlight_symbol(false);

        frame.render_stateful_widget(list, area, &mut self.selected_state);

        // 更新选中文件名 状态
        match self.selected_state.selected() {
            Some(index) => {
                self.selected_file_name = Some(matched_list[index].to_owned());
            }
            None => self.selected_file_name = None,
        }

        Ok(())
    }
}

fn render_no_file_hit(frame: &mut Frame, area: Rect) {
    let centered_area = area.centered(Constraint::Length(15), Constraint::Length(1));
    let paragraph = Paragraph::new("当前路径无文件！").wrap(Wrap { trim: false });
    frame.render_widget(paragraph.centered(), centered_area);
}

/// input为空，则返回全部文件，如果没有匹配项则vec len为0
fn match_file<'a, 'b>(
    input: &'a str,
    vpk_files: &'b indexmap::IndexMap<String, PathBuf>,
) -> Vec<&'b str> {
    if input.is_empty() {
        return vpk_files.keys().map(|v| v.as_str()).collect();
    }

    // 获取阈值
    let threshold = dyn_threadhold(&input);

    let mut matched_list: Vec<(&str, f64)> = vpk_files
        .keys()
        .flat_map(|content| {
            let score = score_keyword(&input, content);
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

/// 根据输出字符串，获得阈值
fn dyn_threadhold(keyword: &str) -> f64 {
    let input_len = keyword.chars().count();
    let mut threshold = 0.5;
    if input_len <= 4 {
        threshold = 0.1 * input_len as f64;
    } else if input_len >= 8 {
        threshold = 0.8;
    }
    threshold
}

/// 给定关键字和文本，返回一个相似度分数（0.0 ~ 1.0）
fn score_keyword(keyword: &str, txt: &str) -> f64 {
    let keyword_lower = keyword.to_lowercase();
    let txt_lower = txt.to_lowercase();

    // 子串命中直接加权（最高优先）
    if txt_lower.contains(&keyword_lower) {
        return 1.0;
    }

    // 计算 jaro-winkler 相似度
    let jw_score = jaro_winkler(&txt_lower, &keyword_lower);

    // 计算 Levenshtein 距离，转成相似度
    let max_len = txt_lower.len().max(keyword_lower.len());
    let lev_dist = levenshtein(&txt_lower, &keyword_lower);
    let lev_score = 1.0 - (lev_dist as f64 / max_len as f64);

    // 这里我们取平均分（可以调整权重）
    (jw_score + lev_score) / 2.0
}

/// Filter vpk_files by matching input against cached map values (substring match).
/// Returns an empty vec if no matches.
/// 用模糊匹配筛选vpk文件，对文件的所有Map值取最高分
fn match_file_by_map<'a, 'b>(
    input: &'a str,
    vpk_files: &'b indexmap::IndexMap<String, PathBuf>,
    map_cache: &'b std::collections::HashMap<String, Vec<String>>,
) -> Vec<&'b str> {
    if input.is_empty() {
        return vpk_files.keys().map(|v| v.as_str()).collect();
    }

    let threshold = dyn_threadhold(&input);

    let mut matched_list: Vec<(&str, f64)> = vpk_files
        .keys()
        .filter_map(|file_name| {
            let map_values = map_cache.get(file_name)?;
            // 取所有Map值中的最高分
            let best_score = map_values
                .iter()
                .map(|v| score_keyword(&input, v))
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

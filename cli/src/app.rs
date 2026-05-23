mod content_layout;
mod event_handler;
mod list_layout;

use super::*;
use crossterm::event::*;
use ratatui::{
    DefaultTerminal, Frame,
    layout::*,
    style::*,
    text::{Line, Span},
    widgets::*,
};
use std::{io, path::PathBuf, time::SystemTime};

#[derive(Debug, Default)]
pub struct App {
    vpk_files: indexmap::IndexMap<String, PathBuf>,
    selected_state: ListState,
    scroll_state: content_layout::ScrollState,
    selected_file_name: Option<String>,
    input_state: String,

    list_layout_percentage: u16,

    show_filter_dialog: bool,
    show_map_dialog: bool,
    show_duplicates: bool,
    display_content: bool,

    use_map_filter: bool,

    map_cache: std::collections::HashMap<String, Vec<String>>,
    duplicate_groups: Vec<Vec<String>>,
    duplicate_list_state: ListState,

    list_line_counts: Vec<u16>,
    list_area_height: u16,

    sort_by_created: bool,
    sort_ascending: bool,
    file_created: std::collections::HashMap<String, SystemTime>,
    file_modified: std::collections::HashMap<String, SystemTime>,

    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        self.init()?;
        while !self.exit {
            terminal.try_draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn init(&mut self) -> color_eyre::Result<()> {
        let (files, created, modified) = get_vpks()?;
        self.vpk_files = files;
        self.file_created = created;
        self.file_modified = modified;
        self.sort_by_created = true;
        self.sort_ascending = false;
        self.sort_files();
        self.selected_state.select(Some(0));
        self.list_layout_percentage = 30;
        Ok(())
    }

    fn page_up(&mut self) {
        let first_visible = self.selected_state.offset() as usize;
        if first_visible == 0 {
            return;
        }
        let height = self.list_area_height.max(1) as usize;
        let mut target = first_visible;
        let mut lines = 0usize;
        while target > 0 {
            let lc = self.list_line_counts.get(target - 1).copied().unwrap_or(1) as usize;
            if lines + lc > height {
                break;
            }
            target -= 1;
            lines += lc;
        }
        *self.selected_state.offset_mut() = target;
        self.selected_state.select(Some(target));
    }

    fn page_down(&mut self) {
        let first_visible = self.selected_state.offset() as usize;
        let height = self.list_area_height.max(1) as usize;
        // 检查最后一项是否已经可见
        let mut lines = 0usize;
        let mut last_visible = first_visible;
        while last_visible < self.list_line_counts.len() {
            let lc = self.list_line_counts[last_visible] as usize;
            if lines + lc > height {
                break;
            }
            lines += lc;
            last_visible += 1;
        }
        if last_visible >= self.list_line_counts.len() {
            return; // 最后一页，不再翻
        }
        // 从当前视口第一项往后跳过 height 行 → 下一页的第一项
        let mut target = first_visible;
        let mut lines = 0usize;
        while target < self.list_line_counts.len() {
            let lc = self.list_line_counts.get(target).copied().unwrap_or(1) as usize;
            if lines + lc > height {
                break;
            }
            lines += lc;
            target += 1;
        }
        target = target.min(self.list_line_counts.len().saturating_sub(1));
        *self.selected_state.offset_mut() = target;
        self.selected_state.select(Some(target));
    }

    fn sort_files(&mut self) {
        let created = &self.file_created;
        let modified = &self.file_modified;
        let by_created = self.sort_by_created;
        let ascending = self.sort_ascending;

        self.vpk_files.sort_by(|k1, _, k2, _| {
            let times = if by_created { created } else { modified };
            let t1 = times
                .get(k1.as_str())
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let t2 = times
                .get(k2.as_str())
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let cmp = t1.cmp(&t2);
            if ascending { cmp } else { cmp.reverse() }
        });
    }

    fn populate_map_cache(&mut self) -> color_eyre::Result<()> {
        if !self.map_cache.is_empty() {
            return Ok(());
        }
        for (file_name, file_path) in &self.vpk_files {
            if let Ok(vpk_info) = vpk::VPKInfo::new(file_path) {
                if let Ok(mission) = vpk_info.get_mission() {
                    let maps = mission::parse_coop_maps(mission);
                    if !maps.is_empty() {
                        self.map_cache.insert(file_name.clone(), maps);
                    }
                }
            }
        }
        Ok(())
    }

    fn find_duplicates(&mut self) {
        self.duplicate_groups.clear();

        // map each map value to files that contain it
        let mut value_to_files: std::collections::HashMap<&str, Vec<&str>> =
            std::collections::HashMap::new();
        for (file_name, map_values) in &self.map_cache {
            for map_value in map_values {
                value_to_files
                    .entry(map_value.as_str())
                    .or_default()
                    .push(file_name.as_str());
            }
        }

        // Union-Find: merge files that share a duplicate map value
        let mut parent: std::collections::HashMap<&str, &str> =
            std::collections::HashMap::new();
        fn find<'a>(
            parent: &mut std::collections::HashMap<&'a str, &'a str>,
            x: &'a str,
        ) -> &'a str {
            let p = parent.get(x).copied().unwrap_or(x);
            if p != x {
                let root = find(parent, p);
                parent.insert(x, root);
                root
            } else {
                x
            }
        }

        for files in value_to_files.values() {
            if files.len() > 1 {
                let root = files[0];
                for &f in &files[1..] {
                    let r1 = find(&mut parent, root);
                    let r2 = find(&mut parent, f);
                    if r1 != r2 {
                        parent.insert(r1, r2);
                    }
                }
            }
        }

        // Collect all files that have at least one duplicate map
        let mut all_files: std::collections::BTreeSet<&str> =
            std::collections::BTreeSet::new();
        for files in value_to_files.values() {
            if files.len() > 1 {
                for &f in files {
                    all_files.insert(f);
                }
            }
        }

        // Collect groups by root
        let mut groups: std::collections::HashMap<&str, Vec<&str>> =
            std::collections::HashMap::new();
        for file in all_files {
            let root = find(&mut parent, file);
            groups.entry(root).or_default().push(file);
        }

        for (_, mut files) in groups {
            if files.len() > 1 {
                files.sort();
                self.duplicate_groups
                    .push(files.into_iter().map(|s| s.to_owned()).collect());
            }
        }
        self.duplicate_groups
            .sort_by(|a, b| a[0].cmp(&b[0]));
        self.duplicate_list_state.select(Some(0));
    }

    fn draw(&mut self, frame: &mut Frame) -> std::io::Result<()> {
        // 先上下布局
        let outer_layout =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(frame.area());
        let [top_area, bottom_area] = [outer_layout[0], outer_layout[1]];

        // 然后在上布局中，嵌套左右布局
        let inner_layout = Layout::horizontal([
            Constraint::Percentage(self.list_layout_percentage),
            Constraint::Fill(1),
        ])
        .split(top_area);
        let [top_left_area, top_right_area] = [inner_layout[0], inner_layout[1]];

        let dialog_open = self.show_filter_dialog
            || self.show_map_dialog
            || self.show_duplicates;
        let filtering = !self.input_state.is_empty();
        let hint_text = if dialog_open {
            " Enter/Esc:关闭   ↑↓:导航 "
        } else if filtering {
            " s:文件过滤  c:地图代码过滤  d:重复文件检测  ctrl+x:清空过滤 "
        } else {
            " s:文件过滤  c:地图代码过滤  d:重复文件检测  ctrl+x:清空过滤\r\n t:创建/修改时间  r:升序/降序"
        };
        frame.render_widget(
            Paragraph::new(format!(
                " q:退出  ctrl+←/→:调整列宽\r\n{}",
                hint_text
            )),
            bottom_area,
        );

        // 渲染列表
        self.render_list(frame, top_left_area)
            .map_err(io::Error::other)?;

        // 渲染内容
        self.render_content(frame, top_right_area)
            .map_err(io::Error::other)?;

        Ok(())
    }
}

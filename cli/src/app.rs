mod content_layout;
mod content_state;
mod event_handler;
mod filter_state;
mod list_layout;
mod list_state;
mod screen;
mod sort_state;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Instant, SystemTime};

use ratatui::{
    DefaultTerminal, Frame,
    layout::*,
    style::{Color, Style},
    widgets::*,
};
use screen::Screen;

use crate::get_vpks;

#[derive(Debug, Default)]
pub struct ScanProgress {
    pub active: bool,
    pub current: usize,
    pub total: usize,
    pub file: String,
}

enum ScanEvent {
    Progress {
        current: usize,
        total: usize,
        file: String,
    },
    MapEntry {
        file_name: String,
        maps: Vec<String>,
    },
    Complete,
}

#[derive(Debug)]
pub struct App {
    pub screen: Screen,
    pub vpk_files: indexmap::IndexMap<String, PathBuf>,
    pub file_created: HashMap<String, SystemTime>,
    pub file_modified: HashMap<String, SystemTime>,
    pub list: list_state::FileListState,
    pub content: content_state::ContentState,
    pub filter: filter_state::FilterState,
    pub sort: sort_state::SortState,
    pub map_cache: HashMap<String, Vec<String>>,
    pub duplicate_groups: Vec<Vec<String>>,
    pub duplicate_list: ratatui::widgets::ListState,
    pub map_codes_list: ratatui::widgets::ListState,
    pub selected_file_name: Option<String>,
    pub list_layout_percentage: u16,
    pub scan_progress: ScanProgress,
    scan_event_rx: Option<mpsc::Receiver<ScanEvent>>,
    pub toast: Option<(String, Instant)>,
    pub exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::default(),
            vpk_files: indexmap::IndexMap::new(),
            file_created: HashMap::new(),
            file_modified: HashMap::new(),
            list: list_state::FileListState::default(),
            content: content_state::ContentState::default(),
            filter: filter_state::FilterState::default(),
            sort: sort_state::SortState::default(),
            map_cache: HashMap::new(),
            duplicate_groups: Vec::new(),
            duplicate_list: ratatui::widgets::ListState::default(),
            map_codes_list: ratatui::widgets::ListState::default(),
            selected_file_name: None,
            list_layout_percentage: 30,
            scan_progress: ScanProgress::default(),
            scan_event_rx: None,
            toast: None,
            exit: false,
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        self.init()?;
        while !self.exit {
            self.process_scan_events();
            terminal.try_draw(|frame| self.draw(frame))?;
            event_handler::handle_events(self)?;
        }
        Ok(())
    }

    fn init(&mut self) -> color_eyre::Result<()> {
        let (files, created, modified) = get_vpks()?;
        self.vpk_files = files;
        self.file_created = created;
        self.file_modified = modified;
        self.sort
            .apply_to(&mut self.vpk_files, &self.file_created, &self.file_modified);
        self.list.select(Some(0));
        self.start_background_scan();
        Ok(())
    }

    fn start_background_scan(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.scan_event_rx = Some(rx);

        let files = self.vpk_files.clone();
        std::thread::spawn(move || {
            let total = files.len();
            if total == 0 {
                let _ = tx.send(ScanEvent::Complete);
                return;
            }
            for (i, (file_name, file_path)) in files.iter().enumerate() {
                let _ = tx.send(ScanEvent::Progress {
                    current: i + 1,
                    total,
                    file: file_name.clone(),
                });

                if let Ok(vpk_info) = crate::vpk::VPKInfo::new(file_path) {
                    if let Ok(mission) = vpk_info.get_mission() {
                        let maps = crate::mission::parse_coop_maps(mission);
                        if !maps.is_empty() {
                            let _ = tx.send(ScanEvent::MapEntry {
                                file_name: file_name.clone(),
                                maps,
                            });
                        }
                    }
                }
            }
            let _ = tx.send(ScanEvent::Complete);
        });
    }

    fn process_scan_events(&mut self) {
        let Some(rx) = &self.scan_event_rx else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(ScanEvent::Progress {
                    current,
                    total,
                    file,
                }) => {
                    self.scan_progress = ScanProgress {
                        active: total > 0,
                        current,
                        total,
                        file,
                    };
                }
                Ok(ScanEvent::MapEntry { file_name, maps }) => {
                    self.map_cache.insert(file_name, maps);
                }
                Ok(ScanEvent::Complete) => {
                    self.scan_progress.active = false;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.scan_progress.active = false;
                    break;
                }
            }
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn find_duplicates(&mut self) {
        self.duplicate_groups.clear();

        let mut value_to_files: HashMap<&str, Vec<&str>> = HashMap::new();
        for (file_name, map_values) in &self.map_cache {
            for map_value in map_values {
                value_to_files
                    .entry(map_value.as_str())
                    .or_default()
                    .push(file_name.as_str());
            }
        }

        let mut parent: HashMap<&str, &str> = HashMap::new();
        fn find<'a>(parent: &mut HashMap<&'a str, &'a str>, x: &'a str) -> &'a str {
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

        let mut all_files: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for files in value_to_files.values() {
            if files.len() > 1 {
                for &f in files {
                    all_files.insert(f);
                }
            }
        }

        let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
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
        self.duplicate_groups.sort_by(|a, b| a[0].cmp(&b[0]));
        self.duplicate_list.select(Some(0));
    }

    fn draw(&mut self, frame: &mut Frame) -> io::Result<()> {
        let outer_layout =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(frame.area());
        let [top_area, bottom_area] = [outer_layout[0], outer_layout[1]];

        let inner_layout = Layout::horizontal([
            Constraint::Percentage(self.list_layout_percentage),
            Constraint::Fill(1),
        ])
        .split(top_area);
        let [top_left_area, top_right_area] = [inner_layout[0], inner_layout[1]];

        // Status bar: hint text (left) + progress bar (right)
        let bottom_layout =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(22)]).split(bottom_area);

        let hint_text: String = match self.screen {
            Screen::MapCodes => " Esc:关闭  ↑↓:选择  Enter:复制 ".to_owned(),
            Screen::Filter(_) | Screen::Duplicates => {
                " Enter/Esc:关闭   ↑↓:导航 ".to_owned()
            }
            _ if !self.filter.input.is_empty() => {
                " s:文件过滤  c:地图代码过滤  d:重复检测  f:地图代码列表 ".to_owned()
            }
            _ => {
                " s:文件过滤  c:地图代码过滤  d:重复检测  f:地图代码列表\r\n t:创建/修改时间  r:升序/降序".to_owned()
            }
        };
        frame.render_widget(
            Paragraph::new(format!(
                " q:退出  ctrl+←/→:调整列宽  ctrl+x:清空过滤\r\n{}",
                hint_text
            )),
            bottom_layout[0],
        );

        // Progress bar (right side of status bar)
        if self.scan_progress.active {
            let percent = if self.scan_progress.total > 0 {
                (self.scan_progress.current * 100 / self.scan_progress.total) as u16
            } else {
                0
            };
            let gauge_area = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(bottom_layout[1]);
            let gauge = Gauge::default()
                .percent(percent)
                .label(format!(" {}% ", percent))
                .gauge_style(Style::default().fg(Color::Green));
            frame.render_widget(gauge, gauge_area[1]);
        }

        // File list (left pane)
        list_layout::render_list(self, frame, top_left_area).map_err(io::Error::other)?;

        // Right pane (by screen)
        match self.screen {
            Screen::Content => {
                content_layout::render_mission(self, frame, top_right_area)
                    .map_err(io::Error::other)?;
            }
            Screen::Filter(_) => {
                content_layout::render_hint(frame, top_right_area);
                content_layout::render_filter_dialog(frame, top_right_area, &self.filter)
                    .map_err(io::Error::other)?;
            }
            Screen::Duplicates => {
                content_layout::render_hint(frame, top_right_area);
                content_layout::render_duplicates_dialog(
                    frame,
                    top_right_area,
                    &self.duplicate_groups,
                    &mut self.duplicate_list,
                )
                .map_err(io::Error::other)?;
            }
            Screen::FileList => {
                content_layout::render_hint(frame, top_right_area);
            }
            Screen::MapCodes => {
                content_layout::render_hint(frame, top_right_area);
                let maps = self
                    .selected_file_name
                    .as_ref()
                    .and_then(|name| self.map_cache.get(name));
                content_layout::render_map_codes_dialog(
                    frame,
                    top_right_area,
                    maps.map(|v| v.as_slice()),
                    &mut self.map_codes_list,
                )
                .map_err(io::Error::other)?;
            }
        }

        // Toast notification overlay (auto-dismiss after 3 seconds)
        if let Some((_, since)) = &self.toast
            && since.elapsed() >= std::time::Duration::from_secs(3)
        {
            self.toast = None;
        }
        if let Some((msg, _)) = &self.toast {
            let text_width = msg.len() as u16 + 4;
            let x = frame.area().width.saturating_sub(text_width) / 2;
            let rect = Rect {
                x,
                y: frame.area().height.saturating_sub(4),
                width: text_width,
                height: 1,
            };
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(msg.as_str())
                    .centered()
                    .style(Style::default().bg(Color::DarkGray).fg(Color::White)),
                rect,
            );
        }

        Ok(())
    }
}

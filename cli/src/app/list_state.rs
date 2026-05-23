use ratatui::widgets::ListState;

#[derive(Debug)]
pub struct FileListState {
    pub selected: ListState,
    pub line_counts: Vec<u16>,
    pub area_height: u16,
}

impl Default for FileListState {
    fn default() -> Self {
        Self {
            selected: ListState::default(),
            line_counts: Vec::new(),
            area_height: 0,
        }
    }
}

impl FileListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected.selected()
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected.select(index);
    }

    pub fn select_previous(&mut self) {
        self.selected.select_previous();
    }

    pub fn select_next(&mut self) {
        self.selected.select_next();
    }

    pub fn select_first(&mut self) {
        self.selected.select_first();
    }

    pub fn select_last(&mut self) {
        self.selected.select_last();
    }

    pub fn offset(&self) -> usize {
        self.selected.offset()
    }

    pub fn offset_mut(&mut self) -> &mut usize {
        self.selected.offset_mut()
    }

    pub fn page_up(&mut self) {
        let first_visible = self.offset();
        if first_visible == 0 {
            return;
        }
        let height = self.area_height.max(1) as usize;
        let mut target = first_visible;
        let mut lines = 0usize;
        while target > 0 {
            let lc = self.line_counts.get(target - 1).copied().unwrap_or(1) as usize;
            if lines + lc > height {
                break;
            }
            target -= 1;
            lines += lc;
        }
        *self.offset_mut() = target;
        self.selected.select(Some(target));
    }

    pub fn page_down(&mut self) {
        let first_visible = self.offset();
        let height = self.area_height.max(1) as usize;
        let mut lines = 0usize;
        let mut last_visible = first_visible;
        while last_visible < self.line_counts.len() {
            let lc = self.line_counts[last_visible] as usize;
            if lines + lc > height {
                break;
            }
            lines += lc;
            last_visible += 1;
        }
        if last_visible >= self.line_counts.len() {
            return;
        }
        let mut target = first_visible;
        let mut lines = 0usize;
        while target < self.line_counts.len() {
            let lc = self.line_counts.get(target).copied().unwrap_or(1) as usize;
            if lines + lc > height {
                break;
            }
            lines += lc;
            target += 1;
        }
        target = target.min(self.line_counts.len().saturating_sub(1));
        *self.offset_mut() = target;
        self.selected.select(Some(target));
    }
}

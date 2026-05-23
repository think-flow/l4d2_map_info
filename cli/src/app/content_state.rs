#[derive(Debug, Default)]
pub struct ContentState {
    pub scroll_offset: u16,
    pub line_count: u16,
    pub area_height: u16,
}

impl ContentState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.get_viewd_offset() <= self.line_count {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.get_botton_offset();
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(self.area_height);
    }

    pub fn scroll_page_down(&mut self) {
        let result = self.scroll_offset.saturating_add(self.area_height);
        let bottom_offset = self.get_botton_offset();
        if result > bottom_offset {
            self.scroll_offset = bottom_offset;
        } else {
            self.scroll_offset = result;
        }
    }

    fn get_botton_offset(&self) -> u16 {
        self.line_count.saturating_sub(self.area_height).saturating_add(1)
    }

    fn get_viewd_offset(&self) -> u16 {
        self.scroll_offset + self.area_height
    }
}

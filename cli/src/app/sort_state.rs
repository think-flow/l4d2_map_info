use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    #[default]
    CreatedAt,
    ModifiedAt,
}

#[derive(Debug)]
pub struct SortState {
    pub field: SortField,
    pub ascending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            field: SortField::default(),
            ascending: false,
        }
    }
}

impl SortState {
    pub fn apply_to(
        &self,
        files: &mut indexmap::IndexMap<String, PathBuf>,
        created: &HashMap<String, SystemTime>,
        modified: &HashMap<String, SystemTime>,
    ) {
        let by_created = self.field == SortField::CreatedAt;
        let times_map = if by_created { created } else { modified };
        files.sort_by(|k1, _, k2, _| {
            let t1 = times_map
                .get(k1.as_str())
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let t2 = times_map
                .get(k2.as_str())
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let cmp = t1.cmp(&t2);
            if self.ascending { cmp } else { cmp.reverse() }
        });
    }

    pub fn toggle_field(&mut self) {
        self.field = match self.field {
            SortField::CreatedAt => SortField::ModifiedAt,
            SortField::ModifiedAt => SortField::CreatedAt,
        };
    }

    pub fn toggle_order(&mut self) {
        self.ascending = !self.ascending;
    }
}

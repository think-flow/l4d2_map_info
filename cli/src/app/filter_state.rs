pub use super::screen::FilterMode;

#[derive(Debug, Default)]
pub struct FilterState {
    pub input: String,
    pub mode: FilterMode,
}

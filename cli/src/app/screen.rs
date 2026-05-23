#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
    FileList,
    Content,
    Filter(FilterMode),
    Duplicates,
    MapCodes,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    #[default]
    FileName,
    MapName,
}

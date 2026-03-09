#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShowNoteKind {
    Html,
    Markdown,
}

impl ShowNoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShowNoteKind::Html => "html",
            ShowNoteKind::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Save,
    ShowHelp,
    StartFilter,
    MoveUp,
    MoveDown,
    SwitchPane,
    FocusPlaylist,
    FocusLibrary,
    ToggleItem,
    Rename,
    Notes,
    ReorderUp,
    ReorderDown,
    LaunchFile,
    LoadPlaylist,
    MoveToLibrary,
    MoveToPlaylist,
    LaunchMpv,
    AddUrl,
    Delete,
    FuzzyNotes,
    EditSources,
    GenerateShowNotes(ShowNoteKind),
}

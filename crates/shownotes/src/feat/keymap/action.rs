/// Output format for generating show notes.
///
/// Specifies the format to use when exporting playlist notes,
/// supporting HTML, Markdown, and plain text output.
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

/// All available actions in the TUI.
///
/// Represents every user action that can be triggered by keybindings,
/// from navigation and pane switching to launching mpv and editing notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Save,
    ShowHelp,
    ShowAlias,
    ShowPath,
    StartFilter,
    MoveUp,
    MoveDown,
    SwitchPane,
    FocusPlaylist,
    FocusLibrary,
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

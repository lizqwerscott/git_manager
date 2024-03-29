#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy)]
pub enum AppAction {
    StartRefresh,
    StartFilter,
    ExitFilter,
    SelectNext,
    SelectPervious,
    SelectEnter,
    SelectCopyPath,
    ComplectionFinish,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineState {
    pub valid: bool,
    /// Write-through lines stay clean.
    pub dirty: bool,
    pub tag: u32,
}

#[derive(Default)]
pub(super) struct Line {
    pub state: LineState,
    pub last_used: u64,
    pub bytes: Vec<u8>,
}

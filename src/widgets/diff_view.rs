pub struct DiffViewState {
    pub content: Option<DiffViewInfo>,
}

impl Default for DiffViewState {
    fn default() -> DiffViewState {
        DiffViewState { content: None }
    }
}

pub struct DiffViewInfo {
    pub diffs: Vec<String>,
    pub scroll: u16,
}
impl DiffViewInfo {
    pub fn new(diffs: Vec<String>) -> Self {
        Self { diffs, scroll: 0 }
    }
}

use git2::Oid;

pub struct DiffViewState {
    pub content: Option<DiffViewInfo>,
}

impl Default for DiffViewState {
    fn default() -> DiffViewState {
        DiffViewState { content: None }
    }
}

pub struct DiffViewInfo {
    pub diffs: Vec<(String, Option<u32>, Option<u32>)>,
    pub oid: Oid,
    pub compare_oid: Oid,
    pub scroll: u16,
}
impl DiffViewInfo {
    pub fn new(diffs: Vec<(String, Option<u32>, Option<u32>)>, oid: Oid, compare_oid: Oid) -> Self {
        Self {
            diffs,
            oid,
            compare_oid,
            scroll: 0,
        }
    }
}

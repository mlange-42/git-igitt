use git2::Oid;
use syntect::highlighting::Style;

#[derive(Default)]
pub struct DiffViewState {
    pub content: Option<DiffViewInfo>,
}

pub struct DiffViewInfo {
    pub diffs: Vec<(String, Option<u32>, Option<u32>)>,
    pub highlighted: Option<Vec<Vec<(Style, String)>>>,
    pub oid: Oid,
    pub compare_oid: Oid,
    pub scroll: (u16, u16),
}
impl DiffViewInfo {
    pub fn new(
        diffs: Vec<(String, Option<u32>, Option<u32>)>,
        highlighted: Option<Vec<Vec<(Style, String)>>>,
        oid: Oid,
        compare_oid: Oid,
    ) -> Self {
        Self {
            diffs,
            highlighted,
            oid,
            compare_oid,
            scroll: (0, 0),
        }
    }
}

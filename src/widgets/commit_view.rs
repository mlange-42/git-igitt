use crate::app::DiffType;
use crate::util::ctrl_chars::CtrlChars;
use crate::widgets::list::{ListItem, StatefulList};
use git2::Oid;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, StatefulWidget, Widget};

#[derive(Default)]
pub struct CommitViewState {
    pub content: Option<CommitViewInfo>,
}

pub struct DiffItem {
    pub(crate) file: String,
    pub(crate) diff_type: DiffType,
}

impl ListItem for DiffItem {
    fn is_selectable(&self) -> bool {
        true
    }
}

pub struct CommitViewInfo {
    pub text: Vec<String>,
    pub diffs: StatefulList<DiffItem>,
    pub oid: Oid,
    pub compare_oid: Oid,
    pub scroll: u16,
}
impl CommitViewInfo {
    pub fn new(
        text: Vec<String>,
        diffs: StatefulList<DiffItem>,
        oid: Oid,
        compare_oid: Oid,
    ) -> Self {
        Self {
            text,
            diffs,
            oid,
            compare_oid,
            scroll: 0,
        }
    }
}

#[derive(Default)]
pub struct CommitView<'a> {
    block: Option<Block<'a>>,
    highlight_symbol: Option<&'a str>,
    style: Style,
}

impl<'a> CommitView<'a> {
    pub fn block(mut self, block: Block<'a>) -> CommitView<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> CommitView<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> CommitView<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }
}

impl<'a> StatefulWidget for CommitView<'a> {
    type State = CommitViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        let (x_start, y0) = (list_area.left(), list_area.top());
        let list_bottom = list_area.top() + list_area.height;

        let max_element_width = list_area.width;
        if let Some(commit_info) = &state.content {
            let scroll = commit_info.scroll;
            let y_start = y0 as i32 - scroll as i32;

            let wrapping =
                textwrap::Options::new(list_area.width as usize).subsequent_indent("        ");
            let ellipsis = &format!(
                "    ...{}",
                " ".repeat(max_element_width.saturating_sub(7) as usize)
            )[..max_element_width as usize];

            let mut y = y_start;
            for (line_idx, text_line) in commit_info.text.iter().enumerate() {
                if text_line.is_empty() {
                    y += 1;
                    if y >= list_bottom as i32 {
                        buf.set_string(x_start, (y - 1) as u16, ellipsis, self.style);
                        break;
                    }
                } else {
                    let wrapped = if line_idx > 1 {
                        #[allow(clippy::needless_borrow)]
                        textwrap::fill(text_line, &wrapping)
                    } else {
                        text_line.clone()
                    };

                    for line in wrapped.lines() {
                        let mut x = x_start;
                        let mut remaining_width = max_element_width;

                        let line_span = CtrlChars::parse(line).into_text();
                        if y >= y0 as i32 {
                            for txt in line_span {
                                for line in txt.lines {
                                    if remaining_width == 0 {
                                        break;
                                    }
                                    let pos = buf.set_spans(x, y as u16, &line, remaining_width);
                                    let w = pos.0.saturating_sub(x);
                                    x = pos.0;
                                    y = pos.1 as i32;
                                    remaining_width = remaining_width.saturating_sub(w);
                                }
                            }
                        }
                        y += 1;
                        if y >= list_bottom as i32 {
                            break;
                        }
                    }
                    if y >= list_bottom as i32 {
                        buf.set_string(x_start, (y - 1) as u16, ellipsis, self.style);
                        break;
                    }
                }
            }
        }
    }
}

impl<'a> Widget for CommitView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = CommitViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

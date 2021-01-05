use crate::app::DiffType;
use crate::widgets::ctrl_chars::CtrlChars;
use crate::widgets::list::StatefulList;
use git2::Oid;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, StatefulWidget, Widget};

pub struct CommitViewState {
    pub content: Option<CommitViewInfo>,
}

impl Default for CommitViewState {
    fn default() -> CommitViewState {
        CommitViewState { content: None }
    }
}

pub struct CommitViewInfo {
    pub text: Vec<String>,
    pub diffs: StatefulList<(String, DiffType, Oid, Oid)>,
    pub oid: Oid,
    pub scroll: u16,
}
impl CommitViewInfo {
    pub fn new(
        text: Vec<String>,
        diffs: StatefulList<(String, DiffType, Oid, Oid)>,
        oid: Oid,
    ) -> Self {
        Self {
            text,
            diffs,
            oid,
            scroll: 0,
        }
    }
}

pub struct CommitView<'a> {
    block: Option<Block<'a>>,
    highlight_symbol: Option<&'a str>,
    style: Style,
}

impl<'a> Default for CommitView<'a> {
    fn default() -> CommitView<'a> {
        CommitView {
            block: None,
            style: Style::default(),
            highlight_symbol: None,
        }
    }
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
                        textwrap::fill(&text_line, &wrapping)
                    } else {
                        text_line.clone()
                    };

                    for line in wrapped.lines() {
                        let mut x = x_start;
                        let mut remaining_width = max_element_width as u16;

                        let line_span = CtrlChars::parse(&line).into_text();
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

        /*
        let list_height = list_area.height as usize;

        let mut start = state.offset;

        let height = std::cmp::min(
            list_height as usize,
            state.text.len().saturating_sub(state.offset),
        );
        let mut end = start + height;

        let selected_row = state.selected.map(|idx| state.indices[idx]);
        let selected = selected_row.unwrap_or(0).min(state.text.len() - 1);

        if selected >= end {
            let diff = selected + 1 - end;
            end += diff;
            start += diff;
        }
        if selected < start {
            let diff = start - selected;
            end -= diff;
            start -= diff;
        }
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = iter::repeat(" ")
            .take(highlight_symbol.width())
            .collect::<String>();

        let style = Style::default();
        for (current_height, (i, item)) in state
            .text
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
            .enumerate()
        {
            let (x, y) = (list_area.left(), list_area.top() + current_height as u16);

            let is_selected = selected_row.map(|s| s == i).unwrap_or(false);
            let elem_x = {
                let symbol = if is_selected {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, style);
                x
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;

            let body = CtrlChars::parse(item).into_text();
            let mut x = elem_x;
            let mut remaining_width = max_element_width as u16;
            for txt in body {
                for line in txt.lines {
                    if remaining_width == 0 {
                        break;
                    }
                    let pos = buf.set_spans(x, y, &line, remaining_width);
                    let w = pos.0.saturating_sub(x);
                    x = pos.0;
                    remaining_width = remaining_width.saturating_sub(w);
                }
            }
        }
         */
    }
}

impl<'a> Widget for CommitView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = CommitViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

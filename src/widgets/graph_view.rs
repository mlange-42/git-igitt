use crate::widgets::ctrl_chars::CtrlChars;
use git_graph::graph::GitGraph;
use std::iter::{self, Iterator};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

pub struct GraphViewState {
    pub graph: Option<GitGraph>,
    pub text: Vec<String>,
    pub indices: Vec<usize>,
    pub offset: usize,
    pub selected: Option<usize>,
}

impl Default for GraphViewState {
    fn default() -> GraphViewState {
        GraphViewState {
            graph: None,
            text: vec![],
            indices: vec![],
            offset: 0,
            selected: None,
        }
    }
}

pub struct GraphView<'a> {
    block: Option<Block<'a>>,
    highlight_symbol: Option<&'a str>,
    style: Style,
}

impl<'a> Default for GraphView<'a> {
    fn default() -> GraphView<'a> {
        GraphView {
            block: None,
            style: Style::default(),
            highlight_symbol: None,
        }
    }
}
impl<'a> GraphView<'a> {
    pub fn block(mut self, block: Block<'a>) -> GraphView<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> GraphView<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> GraphView<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }
}

impl<'a> StatefulWidget for GraphView<'a> {
    type State = GraphViewState;

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

        if state.text.is_empty() {
            return;
        }
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
    }
}

impl<'a> Widget for GraphView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = GraphViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

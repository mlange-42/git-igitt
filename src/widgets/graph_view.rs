use crate::widgets::ctrl_chars::CtrlChars;
use std::iter::{self, Iterator};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

pub struct GraphViewState {
    pub text: Vec<String>,
    pub offset: usize,
    pub selected: Option<usize>,
}

impl Default for GraphViewState {
    fn default() -> GraphViewState {
        GraphViewState {
            text: vec![],
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

impl<'a> GraphView<'a> {
    pub fn new() -> GraphView<'a> {
        GraphView {
            block: None,
            style: Style::default(),
            highlight_symbol: None,
        }
    }

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
        let list_height = list_area.height as u32;

        let mut start = state.offset;
        let mut end = state.offset;
        let mut height = 0;

        for _item in state.text.iter().skip(state.offset) {
            if height + 1 > list_height {
                break;
            }
            height += 1;
            end += 1;
        }

        let selected = state.selected.unwrap_or(0).min(state.text.len() - 1);
        while selected >= end {
            height = height.saturating_add(1);
            end += 1;
            while height > list_height {
                height = height.saturating_sub(1);
                start += 1;
            }
        }

        while selected < start {
            start -= 1;
            height = height.saturating_add(1);
            while height > list_height {
                end -= 1;
                height = height.saturating_sub(1);
            }
        }
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = iter::repeat(" ")
            .take(highlight_symbol.width())
            .collect::<String>();

        let style = Style::default();
        let mut current_height = 0;
        let has_selection = state.selected.is_some();
        for (i, item) in state
            .text
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = {
                let pos = (list_area.left(), list_area.top() + current_height);
                current_height += 1 as u16;
                pos
            };

            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);
            let elem_x = if has_selection {
                let symbol = if is_selected {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, style);
                x
            } else {
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

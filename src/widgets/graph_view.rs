use crate::util::ctrl_chars::CtrlChars;
use crate::widgets::branches_view::BranchItem;
use crate::widgets::list::StatefulList;
use git_graph::graph::GitGraph;
use std::iter::Iterator;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

const SCROLL_MARGIN: usize = 3;
const SCROLLBAR_STR: &str = "\u{2588}";

#[derive(Default)]
pub struct GraphViewState {
    pub graph: Option<GitGraph>,
    pub graph_lines: Vec<String>,
    pub text_lines: Vec<String>,
    pub indices: Vec<usize>,
    pub offset: usize,
    pub selected: Option<usize>,
    pub branches: Option<StatefulList<BranchItem>>,
    pub secondary_selected: Option<usize>,
    pub secondary_changed: bool,
}

impl GraphViewState {
    pub fn move_selection(&mut self, steps: usize, down: bool) -> bool {
        let changed = if let Some(sel) = self.selected {
            let new_idx = if down {
                std::cmp::min(sel.saturating_add(steps), self.indices.len() - 1)
            } else {
                std::cmp::max(sel.saturating_sub(steps), 0)
            };
            self.selected = Some(new_idx);
            new_idx != sel
        } else if !self.graph_lines.is_empty() {
            self.selected = Some(0);
            true
        } else {
            false
        };
        if changed {
            self.secondary_changed = false;
        }
        changed
    }
    pub fn move_secondary_selection(&mut self, steps: usize, down: bool) -> bool {
        let changed = if let Some(sel) = self.secondary_selected {
            let new_idx = if down {
                std::cmp::min(sel.saturating_add(steps), self.indices.len() - 1)
            } else {
                std::cmp::max(sel.saturating_sub(steps), 0)
            };
            self.secondary_selected = Some(new_idx);
            new_idx != sel
        } else if !self.graph_lines.is_empty() {
            if let Some(sel) = self.selected {
                let new_idx = if down {
                    std::cmp::min(sel.saturating_add(steps), self.indices.len() - 1)
                } else {
                    std::cmp::max(sel.saturating_sub(steps), 0)
                };
                self.secondary_selected = Some(new_idx);
                new_idx != sel
            } else {
                false
            }
        } else {
            false
        };
        if changed {
            self.secondary_changed = true;
        }
        changed
    }
}

#[derive(Default)]
pub struct GraphView<'a> {
    block: Option<Block<'a>>,
    highlight_symbol: Option<&'a str>,
    secondary_highlight_symbol: Option<&'a str>,
    style: Style,
    highlight_style: Style,
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

    pub fn highlight_symbol(
        mut self,
        highlight_symbol: &'a str,
        secondary_highlight_symbol: &'a str,
    ) -> GraphView<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self.secondary_highlight_symbol = Some(secondary_highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> GraphView<'a> {
        self.highlight_style = style;
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

        if state.graph_lines.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let mut start = state.offset;

        let height = std::cmp::min(
            list_height,
            state.graph_lines.len().saturating_sub(state.offset),
        );
        let mut end = start + height;

        let selected_row = state.selected.map(|idx| state.indices[idx]);
        let selected = selected_row.unwrap_or(0).min(state.graph_lines.len() - 1);

        let secondary_selected_row = state.secondary_selected.map(|idx| state.indices[idx]);
        let secondary_selected = secondary_selected_row
            .unwrap_or(0)
            .min(state.graph_lines.len() - 1);

        let selected_index = if state.secondary_changed {
            state.secondary_selected.unwrap_or(0)
        } else {
            state.selected.unwrap_or(0)
        };
        let move_to_selected = if state.secondary_changed {
            secondary_selected
        } else {
            selected
        };

        let move_to_end = if selected_index >= state.indices.len() - 1 {
            state.graph_lines.len() - 1
        } else {
            (state.indices[selected_index + 1] - 1).clamp(
                move_to_selected + SCROLL_MARGIN,
                state.graph_lines.len() - 1,
            )
        };
        let move_to_start = move_to_selected.saturating_sub(SCROLL_MARGIN);

        if move_to_end >= end {
            let diff = move_to_end + 1 - end;
            end += diff;
            start += diff;
        }
        if move_to_start < start {
            let diff = start - move_to_start;
            end -= diff;
            start -= diff;
        }
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let secondary_highlight_symbol = self.secondary_highlight_symbol.unwrap_or("");

        let blank_symbol = " ".repeat(highlight_symbol.width());

        let style = Style::default();
        for (current_height, (i, (graph_item, text_item))) in state
            .graph_lines
            .iter()
            .zip(state.text_lines.iter())
            .enumerate()
            .skip(state.offset)
            .take(end - start)
            .enumerate()
        {
            let (x, y) = (list_area.left(), list_area.top() + current_height as u16);

            let is_selected = selected_row.map(|s| s == i).unwrap_or(false);
            let is_sec_selected = secondary_selected_row.map(|s| s == i).unwrap_or(false);
            let elem_x = {
                let symbol = if is_selected {
                    highlight_symbol
                } else if is_sec_selected {
                    secondary_highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, style);
                x
            };

            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: 1,
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;

            let mut body = CtrlChars::parse(graph_item).into_text();
            body.extend(CtrlChars::parse(&format!("  {}", text_item)).into_text());

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

            if is_selected || is_sec_selected {
                buf.set_style(area, self.highlight_style);
            }
        }

        let scroll_start = list_area.top() as usize
            + (((list_height * start) as f32 / state.graph_lines.len() as f32).ceil() as usize)
                .min(list_height - 1);
        let scroll_height = (((list_height * list_height) as f32 / state.graph_lines.len() as f32)
            .floor() as usize)
            .clamp(1, list_height);

        if scroll_height < list_height {
            for y in scroll_start..(scroll_start + scroll_height) {
                buf.set_string(
                    list_area.left() + list_area.width,
                    y as u16,
                    SCROLLBAR_STR,
                    self.style,
                );
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

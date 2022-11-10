use crate::widgets::list::ListState;
use tui::buffer::Buffer;
use tui::layout::{Corner, Rect};
use tui::style::Style;
use tui::text::Span;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

const SCROLL_MARGIN: usize = 2;
const SCROLLBAR_STR: &str = "\u{2588}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileListItem<'a> {
    pub content: Span<'a>,
    pub prefix: Span<'a>,
    pub style: Style,
}

impl<'a> FileListItem<'a> {
    pub fn new<T>(content: T, prefix: T) -> FileListItem<'a>
    where
        T: Into<Span<'a>>,
    {
        FileListItem {
            content: content.into(),
            prefix: prefix.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> FileListItem<'a> {
        self.style = style;
        self
    }
}

#[derive(Debug, Clone)]
pub struct FileList<'a> {
    block: Option<Block<'a>>,
    items: Vec<FileListItem<'a>>,
    /// Style used as a base style for the widget
    style: Style,
    start_corner: Corner,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    highlight_symbol: Option<&'a str>,
}

impl<'a> FileList<'a> {
    pub fn new<T>(items: T) -> FileList<'a>
    where
        T: Into<Vec<FileListItem<'a>>>,
    {
        FileList {
            block: None,
            style: Style::default(),
            items: items.into(),
            start_corner: Corner::TopLeft,
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> FileList<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> FileList<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> FileList<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> FileList<'a> {
        self.highlight_style = style;
        self
    }

    pub fn start_corner(mut self, corner: Corner) -> FileList<'a> {
        self.start_corner = corner;
        self
    }
}

impl<'a> StatefulWidget for FileList<'a> {
    type State = ListState;

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

        if self.items.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let mut start = state.offset;
        let height = std::cmp::min(list_height, self.items.len().saturating_sub(state.offset));
        let mut end = start + height;

        let selected = state.selected.unwrap_or(0).min(self.items.len() - 1);

        let move_to_end = (selected + SCROLL_MARGIN).min(self.items.len() - 1);
        let move_to_start = selected.saturating_sub(SCROLL_MARGIN);

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
        let blank_symbol = " ".repeat(highlight_symbol.width());

        let mut max_scroll = 0;
        let mut current_height = 0;
        for (i, item) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = match self.start_corner {
                Corner::BottomLeft => {
                    current_height += 1;
                    (list_area.left(), list_area.bottom() - current_height)
                }
                _ => {
                    let pos = (list_area.left(), list_area.top() + current_height);
                    current_height += 1;
                    pos
                }
            };
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: 1,
            };
            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);

            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);
            let elem_x = {
                let symbol = if is_selected {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, item_style);
                x
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;
            let max_width_2 = max_element_width.saturating_sub(item.prefix.width());

            buf.set_span(elem_x, y, &item.prefix, max_element_width as u16);
            if state.scroll_x > 0 && item.content.content.width() > max_width_2 {
                if item.content.content.width() - max_width_2 > max_scroll {
                    max_scroll = item.content.content.width() - max_width_2;
                }

                let start = std::cmp::min(
                    item.content.content.width().saturating_sub(max_width_2) + 2,
                    std::cmp::min(item.content.content.width(), state.scroll_x as usize + 2),
                );
                let span = Span::styled(
                    format!("..{}", &item.content.content[start..]),
                    item.content.style,
                );
                buf.set_span(
                    elem_x + item.prefix.width() as u16,
                    y,
                    &span,
                    max_width_2 as u16,
                );
            } else {
                buf.set_span(
                    elem_x + item.prefix.width() as u16,
                    y,
                    &item.content,
                    max_width_2 as u16,
                );
            }
            if is_selected {
                buf.set_style(area, self.highlight_style);
            }
        }
        if state.scroll_x > max_scroll as u16 {
            state.scroll_x = max_scroll as u16;
        }

        let scroll_start = list_area.top() as usize
            + (((list_height * start) as f32 / self.items.len() as f32).ceil() as usize)
                .min(list_height - 1);
        let scroll_height = (((list_height * list_height) as f32 / self.items.len() as f32).floor()
            as usize)
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

impl<'a> Widget for FileList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

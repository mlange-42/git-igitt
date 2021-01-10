use crate::widgets::list::ListState;
use tui::buffer::Buffer;
use tui::layout::{Corner, Rect};
use tui::style::Style;
use tui::text::Span;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn height(&self) -> usize {
        1
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
        let mut end = state.offset;
        let mut height = 0;
        for item in self.items.iter().skip(state.offset) {
            if height + item.height() > list_height {
                break;
            }
            height += item.height();
            end += 1;
        }

        let selected = state.selected.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            height = height.saturating_add(self.items[end].height());
            end += 1;
            while height > list_height {
                height = height.saturating_sub(self.items[start].height());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            height = height.saturating_add(self.items[start].height());
            while height > list_height {
                end -= 1;
                height = height.saturating_sub(self.items[end].height());
            }
        }
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = std::iter::repeat(" ")
            .take(highlight_symbol.width())
            .collect::<String>();

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
                    current_height += item.height() as u16;
                    (list_area.left(), list_area.bottom() - current_height)
                }
                _ => {
                    let pos = (list_area.left(), list_area.top() + current_height);
                    current_height += item.height() as u16;
                    pos
                }
            };
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: item.height() as u16,
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

            buf.set_span(elem_x, y as u16, &item.prefix, max_element_width as u16);
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
                    y as u16,
                    &span,
                    max_width_2 as u16,
                );
            } else {
                buf.set_span(
                    elem_x + item.prefix.width() as u16,
                    y as u16,
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
    }
}

impl<'a> Widget for FileList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

use crate::widgets::list::{ListItem, ListState};
use tui::buffer::Buffer;
use tui::layout::{Corner, Rect};
use tui::style::Style;
use tui::text::Text;
use tui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq)]
pub enum BranchItemType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Heading,
}

impl BranchItemType {
    pub fn is_selectable(&self) -> bool {
        self != &BranchItemType::Heading
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchListItem<'a> {
    pub content: Text<'a>,
    pub style: Style,
    pub item_type: &'a BranchItemType,
}

impl<'a> BranchListItem<'a> {
    pub fn new<T>(content: T, item_type: &'a BranchItemType) -> BranchListItem<'a>
    where
        T: Into<Text<'a>>,
    {
        BranchListItem {
            content: content.into(),
            style: Style::default(),
            item_type,
        }
    }

    pub fn style(mut self, style: Style) -> BranchListItem<'a> {
        self.style = style;
        self
    }

    pub fn height(&self) -> usize {
        self.content.height()
    }
}

pub struct BranchItem {
    pub(crate) name: String,
    pub(crate) color: u8,
    pub(crate) branch_type: BranchItemType,
}

impl BranchItem {
    pub fn new(name: String, color: u8, branch_type: BranchItemType) -> Self {
        Self {
            name,
            color,
            branch_type,
        }
    }
}

impl ListItem for BranchItem {
    fn is_selectable(&self) -> bool {
        self.branch_type != BranchItemType::Heading
    }
}

impl<'a> ListItem for BranchListItem<'a> {
    fn is_selectable(&self) -> bool {
        self.item_type != &BranchItemType::Heading
    }
}

#[derive(Debug, Clone)]
pub struct BranchList<'a> {
    block: Option<Block<'a>>,
    items: Vec<BranchListItem<'a>>,
    style: Style,
    start_corner: Corner,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    highlight_symbol: Option<&'a str>,
}

impl<'a> BranchList<'a> {
    pub fn new<T>(items: T) -> BranchList<'a>
    where
        T: Into<Vec<BranchListItem<'a>>>,
    {
        BranchList {
            block: None,
            style: Style::default(),
            items: items.into(),
            start_corner: Corner::TopLeft,
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> BranchList<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> BranchList<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> BranchList<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> BranchList<'a> {
        self.highlight_style = style;
        self
    }

    pub fn start_corner(mut self, corner: Corner) -> BranchList<'a> {
        self.start_corner = corner;
        self
    }
}

impl<'a> StatefulWidget for BranchList<'a> {
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
            let elem_x = if item.is_selectable() {
                let symbol = if is_selected {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, item_style);
                x
            } else {
                x
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;
            for (j, line) in item.content.lines.iter().enumerate() {
                buf.set_spans(elem_x, y + j as u16, line, max_element_width as u16);
            }
            if is_selected {
                buf.set_style(area, self.highlight_style);
            }
        }
    }
}

impl<'a> Widget for BranchList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

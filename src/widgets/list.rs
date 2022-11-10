use tui::style::Style;
use tui::text::Text;

#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub offset: usize,
    pub selected: Option<usize>,
    pub scroll_x: u16,
}

impl ListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}

pub trait ListItem {
    fn is_selectable(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultListItem<'a> {
    pub content: Text<'a>,
    pub style: Style,
}

impl<'a> DefaultListItem<'a> {
    pub fn new<T>(content: T) -> DefaultListItem<'a>
    where
        T: Into<Text<'a>>,
    {
        DefaultListItem {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> DefaultListItem<'a> {
        self.style = style;
        self
    }

    pub fn height(&self) -> usize {
        self.content.height()
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T: ListItem> Default for StatefulList<T> {
    fn default() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }
}

impl<T: ListItem> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn fwd(&mut self, steps: usize) -> bool {
        match self.state.selected() {
            Some(sel) => {
                for _ in 0..steps {
                    if !self.next() {
                        break;
                    }
                }
                if let Some(new_sel) = self.state.selected() {
                    sel != new_sel
                } else {
                    true
                }
            }
            None => self.next(),
        }
    }

    pub fn bwd(&mut self, steps: usize) -> bool {
        match self.state.selected() {
            Some(sel) => {
                for _ in 0..steps {
                    if !self.previous() {
                        break;
                    }
                }
                if let Some(new_sel) = self.state.selected() {
                    sel != new_sel
                } else {
                    true
                }
            }
            None => self.next(),
        }
    }

    fn next(&mut self) -> bool {
        match self.state.selected() {
            Some(i) => {
                if i < self.items.len() - 1 {
                    for (idx, item) in self.items.iter().enumerate().skip(i + 1) {
                        if item.is_selectable() {
                            self.state.select(Some(idx));
                            return true;
                        }
                    }
                }
            }
            None => {
                for (idx, item) in self.items.iter().enumerate() {
                    if item.is_selectable() {
                        self.state.select(Some(idx));
                        return true;
                    }
                }
            }
        };
        false
    }

    fn previous(&mut self) -> bool {
        match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    for (idx, item) in self
                        .items
                        .iter()
                        .enumerate()
                        .rev()
                        .skip(self.items.len() - i)
                    {
                        if item.is_selectable() {
                            self.state.select(Some(idx));
                            return true;
                        }
                    }
                }
            }
            None => {
                for (idx, item) in self.items.iter().enumerate() {
                    if item.is_selectable() {
                        self.state.select(Some(idx));
                        return true;
                    }
                }
            }
        };
        false
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

use crate::widgets::graph_view::GraphViewState;

pub struct App<'a> {
    pub graph_state: GraphViewState,
    pub title: &'a str,
    pub enhanced_graphics: bool,
    pub should_quit: bool,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> App<'a> {
        App {
            graph_state: GraphViewState::default(),
            title,
            enhanced_graphics,
            should_quit: false,
        }
    }
    pub fn with_graph(mut self, text: Vec<String>, indices: Vec<usize>) -> App<'a> {
        self.graph_state.text = text;
        self.graph_state.indices = indices;
        self
    }

    pub fn on_up(&mut self) {
        if let Some(sel) = self.graph_state.selected {
            if sel > 0 {
                self.graph_state.selected = Some(sel - 1);
            }
        } else {
            if !self.graph_state.text.is_empty() {
                self.graph_state.selected = Some(0);
            }
        }
    }

    pub fn on_down(&mut self) {
        if let Some(sel) = self.graph_state.selected {
            if sel < self.graph_state.indices.len() - 1 {
                self.graph_state.selected = Some(sel + 1);
            }
        } else {
            if !self.graph_state.text.is_empty() {
                self.graph_state.selected = Some(0);
            }
        }
    }
}

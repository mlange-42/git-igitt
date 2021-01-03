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
    pub fn with_graph(mut self, text: Vec<String>) -> App<'a> {
        self.graph_state.text = text;
        self
    }
}

use tui::widgets::ListState;

pub struct ModelListState {
    pub models: Vec<String>,
    pub color: bool,
    pub state: ListState,
}

impl ModelListState {
    pub fn new(models: Vec<String>, color: bool) -> ModelListState {
        ModelListState {
            models,
            color,
            state: ListState::default(),
        }
    }

    pub fn fwd(&mut self, steps: usize) {
        let i = match self.state.selected() {
            Some(i) => std::cmp::min(i.saturating_add(steps), self.models.len() - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn bwd(&mut self, steps: usize) {
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(steps),
            None => 0,
        };
        self.state.select(Some(i));
    }
}

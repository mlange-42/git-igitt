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

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.models.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.models.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn on_up(&mut self) {
        self.previous()
    }

    pub fn on_down(&mut self) {
        self.next()
    }
}

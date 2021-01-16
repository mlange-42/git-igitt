#[derive(Clone)]
pub struct AppSettings {
    pub tab_spaces: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            tab_spaces: "    ".to_string(),
        }
    }
}

impl AppSettings {
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab_spaces = " ".repeat(width);
        self
    }
}

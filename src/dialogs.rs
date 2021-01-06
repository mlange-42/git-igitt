use std::io::Error;
use std::path::PathBuf;
use tui::widgets::ListState;

pub struct FileDialog<'a> {
    pub title: &'a str,
    pub location: PathBuf,
    pub selection: Option<PathBuf>,
    pub dirs: Vec<String>,
    pub error_message: Option<String>,
    pub color: bool,
    pub state: ListState,
}

impl<'a> FileDialog<'a> {
    pub fn new(title: &'a str, color: bool) -> Result<FileDialog<'a>, Error> {
        Ok(FileDialog {
            title,
            location: std::env::current_dir()?,
            selection: None,
            dirs: vec![],
            error_message: None,
            color,
            state: ListState::default(),
        })
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.dirs.len() - 1 {
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
                    self.dirs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn dismiss_error(&mut self) {
        self.error_message = None;
    }

    pub fn on_up(&mut self) {
        self.previous()
    }

    pub fn on_down(&mut self) {
        self.next()
    }

    pub fn on_left(&mut self) -> Result<(), String> {
        if let Some(par) = self.location.parent() {
            self.location = PathBuf::from(par);
            self.selection_changed()?;
        }
        Ok(())
    }

    pub fn on_right(&mut self) -> Result<(), String> {
        if let Some(sel) = self.state.selected() {
            let file = &self.dirs[sel];
            let mut path = PathBuf::from(&self.location);
            path.push(file);
            self.location = path;
            self.selection_changed()?;
        }
        Ok(())
    }

    pub fn on_enter(&mut self) {
        if let Some(sel) = self.state.selected() {
            let file = &self.dirs[sel];
            let mut path = PathBuf::from(&self.location);
            path.push(file);
            self.selection = Some(path);
        }
    }

    pub fn selection_changed(&mut self) -> Result<(), String> {
        self.dirs = std::fs::read_dir(&self.location)
            .map_err(|err| err.to_string())?
            .filter_map(|path| match path {
                Ok(path) => {
                    if path.path().is_dir() {
                        path.path()
                            .components()
                            .last()
                            .and_then(|c| c.as_os_str().to_str().and_then(|s| Some(s.to_string())))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .collect();
        if self.dirs.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
        Ok(())
    }
}

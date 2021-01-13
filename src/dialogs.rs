use crate::app::App;
use git2::Repository;
use std::io::Error;
use std::path::PathBuf;
use tui::widgets::ListState;

pub struct FileDialog<'a> {
    pub title: &'a str,
    pub location: PathBuf,
    pub selection: Option<PathBuf>,
    pub dirs: Vec<(String, bool)>,
    pub error_message: Option<String>,
    pub color: bool,
    pub state: ListState,
    pub previous_app: Option<App>,
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
            previous_app: None,
        })
    }

    pub fn fwd(&mut self, steps: usize) {
        let i = match self.state.selected() {
            Some(i) => std::cmp::min(i.saturating_add(steps), self.dirs.len() - 1),
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

    pub fn on_up(&mut self, is_shift: bool) {
        let step = if is_shift { 10 } else { 1 };
        self.bwd(step)
    }

    pub fn on_down(&mut self, is_shift: bool) {
        let step = if is_shift { 10 } else { 1 };
        self.fwd(step)
    }

    pub fn on_left(&mut self) -> Result<(), String> {
        if let Some(par) = self.location.parent() {
            let temp_path = self.location.clone();
            let prev = self.location.clone();
            let path = PathBuf::from(par);
            self.location = path.clone();
            match self.selection_changed(Some(prev)) {
                Ok(_) => {}
                Err(err) => {
                    self.location = temp_path;
                    self.error_message = Some(format!(
                        "Problem entering directory {}\n{}",
                        path.display(),
                        err
                    ));
                }
            };
        }
        Ok(())
    }

    pub fn on_right(&mut self) -> Result<(), String> {
        if let Some(sel) = self.state.selected() {
            if sel == 0 {
                return self.on_left();
            }
            let temp_path = self.location.clone();
            let file = &self.dirs[sel];
            let mut path = PathBuf::from(&self.location);
            path.push(&file.0);
            self.location = path.clone();
            match self.selection_changed(None) {
                Ok(_) => {}
                Err(err) => {
                    self.location = temp_path;
                    self.error_message = Some(format!(
                        "Problem entering directory {}\n{}",
                        path.display(),
                        err
                    ));
                }
            };
        }
        Ok(())
    }

    pub fn on_enter(&mut self) {
        if let Some(sel) = self.state.selected() {
            let file = &self.dirs[sel];
            let mut path = PathBuf::from(&self.location);
            path.push(&file.0);
            self.selection = Some(path);
        }
    }

    pub fn selection_changed(&mut self, prev_location: Option<PathBuf>) -> Result<(), String> {
        self.dirs = std::fs::read_dir(&self.location)
            .map_err(|err| err.to_string())?
            .filter_map(|path| match path {
                Ok(path) => {
                    if path.path().is_dir() {
                        let is_repo = Repository::open(path.path()).is_ok();
                        path.path()
                            .components()
                            .last()
                            .and_then(|c| c.as_os_str().to_str().map(|s| (s.to_string(), is_repo)))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .collect();
        self.dirs.insert(0, ("..".to_string(), false));

        if self.dirs.is_empty() {
            self.state.select(None);
        } else if let Some(prev) = prev_location {
            if let Some(prev_index) = prev
                .components()
                .last()
                .and_then(|comp| comp.as_os_str().to_str())
                .and_then(|dir| self.dirs.iter().position(|d| d.0 == dir))
            {
                self.state.select(Some(prev_index));
            } else {
                self.state.select(Some(0));
            }
        } else {
            self.state.select(Some(0));
        }
        Ok(())
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

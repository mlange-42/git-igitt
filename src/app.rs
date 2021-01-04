use crate::widgets::graph_view::GraphViewState;
use git2::Oid;
use git_graph::graph::GitGraph;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::Settings;

#[derive(PartialEq)]
pub enum ActiveView {
    Graph,
    Commit,
    Diff,
    Help(u16),
}

pub type CurrentBranches = Vec<(Option<String>, Option<Oid>)>;

pub struct App<'a> {
    pub graph_state: GraphViewState,
    pub title: &'a str,
    pub active_view: ActiveView,
    pub prev_active_view: Option<ActiveView>,
    pub curr_branches: Vec<(Option<String>, Option<Oid>)>,
    pub is_fullscreen: bool,
    pub enhanced_graphics: bool,
    pub should_quit: bool,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> App<'a> {
        App {
            graph_state: GraphViewState::default(),
            title,
            active_view: ActiveView::Graph,
            prev_active_view: None,
            curr_branches: vec![],
            is_fullscreen: false,
            enhanced_graphics,
            should_quit: false,
        }
    }

    pub fn with_graph(
        mut self,
        graph: GitGraph,
        text: Vec<String>,
        indices: Vec<usize>,
    ) -> App<'a> {
        self.graph_state.graph = Some(graph);
        self.graph_state.text = text;
        self.graph_state.indices = indices;
        self
    }

    pub fn with_branches(mut self, branches: Vec<(Option<String>, Option<Oid>)>) -> App<'a> {
        self.curr_branches = branches;
        self
    }

    pub fn clear_graph(mut self) -> App<'a> {
        self.graph_state.graph = None;
        self.graph_state.text = vec![];
        self.graph_state.indices = vec![];
        self
    }

    pub fn reload(
        mut self,
        settings: &Settings,
        max_commits: Option<usize>,
    ) -> Result<App<'a>, String> {
        let selected = self.graph_state.selected;
        let mut temp = None;
        std::mem::swap(&mut temp, &mut self.graph_state.graph);
        if let Some(graph) = temp {
            let sel_oid = selected
                .and_then(|idx| graph.commits.get(idx))
                .map(|info| info.oid);
            let repo = graph.take_repository();
            let graph = GitGraph::new(repo, &settings, max_commits)?;
            let (lines, indices) = print_unicode(&graph, &settings)?;

            let sel_idx = sel_oid.and_then(|oid| graph.indices.get(&oid)).cloned();
            self.graph_state.selected = sel_idx;
            Ok(self.with_graph(graph, lines, indices))
        } else {
            Ok(self)
        }
    }

    pub fn on_up(&mut self, is_shift: bool) {
        match self.active_view {
            ActiveView::Graph => {
                let step = if is_shift { 10 } else { 1 };
                if let Some(sel) = self.graph_state.selected {
                    self.graph_state.selected = Some(std::cmp::max(sel.saturating_sub(step), 0));
                } else if !self.graph_state.text.is_empty() {
                    self.graph_state.selected = Some(0);
                }
            }
            ActiveView::Help(scroll) => {
                self.active_view = ActiveView::Help(scroll.saturating_sub(1))
            }
            _ => {}
        }
    }

    pub fn on_down(&mut self, is_shift: bool) {
        match self.active_view {
            ActiveView::Graph => {
                let step = if is_shift { 10 } else { 1 };
                if let Some(sel) = self.graph_state.selected {
                    self.graph_state.selected = Some(std::cmp::min(
                        sel.saturating_add(step),
                        self.graph_state.indices.len() - 1,
                    ));
                } else if !self.graph_state.indices.is_empty() {
                    self.graph_state.selected = Some(0);
                }
            }
            ActiveView::Help(scroll) => {
                self.active_view = ActiveView::Help(scroll.saturating_add(1))
            }
            _ => {}
        }
    }

    pub fn on_home(&mut self) {
        if let ActiveView::Graph = self.active_view {
            if !self.graph_state.text.is_empty() {
                self.graph_state.selected = Some(0);
            }
        }
    }

    pub fn on_end(&mut self) {
        if let ActiveView::Graph = self.active_view {
            if !self.graph_state.indices.is_empty() {
                self.graph_state.selected = Some(self.graph_state.indices.len() - 1);
            }
        }
    }
    pub fn on_right(&mut self) {
        self.active_view = match &self.active_view {
            ActiveView::Graph => ActiveView::Commit,
            ActiveView::Commit => ActiveView::Diff,
            ActiveView::Diff => ActiveView::Diff,
            ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
        }
    }
    pub fn on_left(&mut self) {
        self.active_view = match &self.active_view {
            ActiveView::Graph => ActiveView::Graph,
            ActiveView::Commit => ActiveView::Graph,
            ActiveView::Diff => ActiveView::Commit,
            ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
        }
    }

    pub fn on_tab(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;
    }

    pub fn on_esc(&mut self) {
        if let ActiveView::Help(_) = self.active_view {
            self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
        } else {
            self.active_view = ActiveView::Graph;
            self.is_fullscreen = false;
        }
    }

    pub fn show_help(&mut self) {
        if let ActiveView::Help(_) = self.active_view {
        } else {
            let mut temp = ActiveView::Help(0);
            std::mem::swap(&mut temp, &mut self.active_view);
            self.prev_active_view = Some(temp);
        }
    }
}

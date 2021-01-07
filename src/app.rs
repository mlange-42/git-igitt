use crate::widgets::commit_view::{CommitViewInfo, CommitViewState};
use crate::widgets::diff_view::{DiffViewInfo, DiffViewState};
use crate::widgets::files_view::StatefulList;
use crate::widgets::graph_view::GraphViewState;
use crate::widgets::models_view::ModelListState;
use git2::{DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, Oid};
use git_graph::config::get_available_models;
use git_graph::graph::GitGraph;
use git_graph::print::unicode::{format_branches, print_unicode};
use git_graph::settings::Settings;
use std::fmt::{Error, Write};
use std::path::PathBuf;
use std::str::FromStr;
use tui::style::Color;

const HASH_COLOR: u8 = 11;

#[derive(PartialEq)]
pub enum ActiveView {
    Graph,
    Commit,
    Files,
    Diff,
    Models,
    Help(u16),
}

pub enum DiffType {
    Added,
    Deleted,
    Modified,
    Renamed,
}

impl FromStr for DiffType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tp = match s {
            "A" => DiffType::Added,
            "D" => DiffType::Deleted,
            "M" => DiffType::Modified,
            "R" => DiffType::Renamed,
            other => return Err(format!("Unknown diff type {}", other)),
        };
        Ok(tp)
    }
}

impl ToString for DiffType {
    fn to_string(&self) -> String {
        match self {
            DiffType::Added => "+",
            DiffType::Deleted => "-",
            DiffType::Modified => "m",
            DiffType::Renamed => "r",
        }
        .to_string()
    }
}

impl DiffType {
    pub fn to_color(&self) -> Color {
        match self {
            DiffType::Added => Color::LightGreen,
            DiffType::Deleted => Color::LightRed,
            DiffType::Modified => Color::LightYellow,
            DiffType::Renamed => Color::LightBlue,
        }
    }
}

pub type CurrentBranches = Vec<(Option<String>, Option<Oid>)>;

pub struct App {
    pub graph_state: GraphViewState,
    pub commit_state: CommitViewState,
    pub diff_state: DiffViewState,
    pub models_state: Option<ModelListState>,
    pub title: String,
    pub repo_name: String,
    pub active_view: ActiveView,
    pub prev_active_view: Option<ActiveView>,
    pub curr_branches: Vec<(Option<String>, Option<Oid>)>,
    pub is_fullscreen: bool,
    pub horizontal_split: bool,
    pub color: bool,
    pub should_quit: bool,
    pub models_path: PathBuf,
}

impl App {
    pub fn new(title: String, repo_name: String, models_path: PathBuf) -> App {
        App {
            graph_state: GraphViewState::default(),
            commit_state: CommitViewState::default(),
            diff_state: DiffViewState::default(),
            models_state: None,
            title,
            repo_name,
            active_view: ActiveView::Graph,
            prev_active_view: None,
            curr_branches: vec![],
            is_fullscreen: false,
            horizontal_split: true,
            color: true,
            should_quit: false,
            models_path,
        }
    }

    pub fn with_graph(mut self, graph: GitGraph, text: Vec<String>, indices: Vec<usize>) -> App {
        self.graph_state.graph = Some(graph);
        self.graph_state.text = text;
        self.graph_state.indices = indices;
        self
    }

    pub fn with_branches(mut self, branches: Vec<(Option<String>, Option<Oid>)>) -> App {
        self.curr_branches = branches;
        self
    }

    pub fn with_color(mut self, color: bool) -> App {
        self.color = color;
        self
    }

    pub fn clear_graph(mut self) -> App {
        self.graph_state.graph = None;
        self.graph_state.text = vec![];
        self.graph_state.indices = vec![];
        self
    }

    pub fn reload(
        mut self,
        settings: &Settings,
        max_commits: Option<usize>,
    ) -> Result<App, String> {
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
            let old_idx = self.graph_state.selected;
            self.graph_state.selected = sel_idx;
            if sel_idx.is_some() != old_idx.is_some() {
                self.selection_changed()?;
            }
            Ok(self.with_graph(graph, lines, indices))
        } else {
            Ok(self)
        }
    }

    pub fn on_up(&mut self, is_shift: bool, is_ctrl: bool) -> Result<(), String> {
        let step = if is_shift { 10 } else { 1 };
        match self.active_view {
            ActiveView::Graph => {
                if is_ctrl {
                    if self.graph_state.move_secondary_selection(step, false) {
                        self.selection_changed()?;
                    }
                } else if self.graph_state.move_selection(step, false) {
                    self.selection_changed()?;
                }
            }
            ActiveView::Help(scroll) => {
                self.active_view = ActiveView::Help(scroll.saturating_sub(step as u16))
            }
            ActiveView::Commit => {
                if let Some(content) = &mut self.commit_state.content {
                    content.scroll = content.scroll.saturating_sub(step as u16);
                }
            }
            ActiveView::Files => {
                if let Some(content) = &mut self.commit_state.content {
                    content.diffs.bwd(step);
                    self.file_changed()?;
                }
            }
            ActiveView::Diff => {
                if let Some(content) = &mut self.diff_state.content {
                    content.scroll = content.scroll.saturating_sub(step as u16);
                }
            }
            ActiveView::Models => {
                if let Some(state) = &mut self.models_state {
                    state.bwd(step)
                }
            }
        }
        Ok(())
    }

    pub fn on_down(&mut self, is_shift: bool, is_ctrl: bool) -> Result<(), String> {
        let step = if is_shift { 10 } else { 1 };
        match self.active_view {
            ActiveView::Graph => {
                if is_ctrl {
                    if self.graph_state.move_secondary_selection(step, true) {
                        self.selection_changed()?;
                    }
                } else if self.graph_state.move_selection(step, true) {
                    self.selection_changed()?;
                }
            }
            ActiveView::Help(scroll) => {
                self.active_view = ActiveView::Help(scroll.saturating_add(step as u16))
            }
            ActiveView::Commit => {
                if let Some(content) = &mut self.commit_state.content {
                    content.scroll = content.scroll.saturating_add(step as u16);
                }
            }
            ActiveView::Files => {
                if let Some(content) = &mut self.commit_state.content {
                    content.diffs.fwd(step);
                    self.file_changed()?;
                }
            }
            ActiveView::Diff => {
                if let Some(content) = &mut self.diff_state.content {
                    content.scroll = content.scroll.saturating_add(step as u16);
                }
            }
            ActiveView::Models => {
                if let Some(state) = &mut self.models_state {
                    state.fwd(step)
                }
            }
        }
        Ok(())
    }

    pub fn on_home(&mut self) -> Result<(), String> {
        if let ActiveView::Graph = self.active_view {
            if !self.graph_state.text.is_empty() {
                self.graph_state.selected = Some(0);
                self.selection_changed()?;
            }
        }
        Ok(())
    }

    pub fn on_end(&mut self) -> Result<(), String> {
        if let ActiveView::Graph = self.active_view {
            if !self.graph_state.indices.is_empty() {
                self.graph_state.selected = Some(self.graph_state.indices.len() - 1);
                self.selection_changed()?;
            }
        }
        Ok(())
    }
    pub fn on_right(&mut self) {
        self.active_view = match &self.active_view {
            ActiveView::Graph => ActiveView::Commit,
            ActiveView::Commit => ActiveView::Files,
            ActiveView::Files => ActiveView::Diff,
            ActiveView::Diff => ActiveView::Diff,
            ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
            ActiveView::Models => ActiveView::Models,
        }
    }
    pub fn on_left(&mut self) {
        self.active_view = match &self.active_view {
            ActiveView::Graph => ActiveView::Graph,
            ActiveView::Commit => ActiveView::Graph,
            ActiveView::Files => ActiveView::Commit,
            ActiveView::Diff => ActiveView::Files,
            ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
            ActiveView::Models => ActiveView::Models,
        }
    }

    pub fn on_enter(&mut self) -> Result<(), String> {
        match &self.active_view {
            ActiveView::Help(_) => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph)
            }
            _ => {
                if self.graph_state.secondary_selected.is_some() {
                    self.graph_state.secondary_selected = None;
                    self.graph_state.secondary_changed = false;
                    self.selection_changed()?;
                }
            }
        }
        Ok(())
    }

    pub fn on_tab(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;
    }

    pub fn on_esc(&mut self) {
        if let ActiveView::Help(_) = self.active_view {
            self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
        } else if let ActiveView::Models = self.active_view {
            self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
        } else {
            self.active_view = ActiveView::Graph;
            self.is_fullscreen = false;
        }
    }

    pub fn toggle_layout(&mut self) {
        self.horizontal_split = !self.horizontal_split;
    }

    pub fn show_help(&mut self) {
        if let ActiveView::Help(_) = self.active_view {
        } else {
            let mut temp = ActiveView::Help(0);
            std::mem::swap(&mut temp, &mut self.active_view);
            self.prev_active_view = Some(temp);
        }
    }

    pub fn select_model(&mut self) -> Result<(), String> {
        if let ActiveView::Models = self.active_view {
        } else {
            let mut temp = ActiveView::Models;
            std::mem::swap(&mut temp, &mut self.active_view);
            self.prev_active_view = Some(temp);

            let models = get_available_models(&self.models_path)?;
            self.models_state = Some(ModelListState::new(models, self.color));
        }
        Ok(())
    }

    fn selection_changed(&mut self) -> Result<(), String> {
        if let Some(graph) = &self.graph_state.graph {
            self.commit_state.content =
                if let Some((info, idx)) = self.graph_state.selected.and_then(move |sel_idx| {
                    graph.commits.get(sel_idx).map(|commit| (commit, sel_idx))
                }) {
                    let commit = graph
                        .repository
                        .find_commit(info.oid)
                        .map_err(|err| err.message().to_string())?;

                    let head_idx = graph.indices.get(&graph.head.oid);
                    let head = if head_idx.map_or(false, |h| h == &idx) {
                        Some(&graph.head)
                    } else {
                        None
                    };

                    let hash_color = if self.color { Some(HASH_COLOR) } else { None };
                    let branches = format_branches(&graph, info, head, self.color)?;
                    let message_fmt = crate::util::format::format(&commit, branches, hash_color)?;

                    let compare_to = if let Some(sel) = self.graph_state.secondary_selected {
                        let sec_selected_info = graph.commits.get(sel);
                        if let Some(info) = sec_selected_info {
                            Some(
                                graph
                                    .repository
                                    .find_commit(info.oid)
                                    .map_err(|err| err.message().to_string())?,
                            )
                        } else {
                            commit.parent(0).ok()
                        }
                    } else {
                        commit.parent(0).ok()
                    };
                    let comp_oid = compare_to.as_ref().map(|c| c.id());

                    let mut diffs = vec![];
                    let diff = graph
                        .repository
                        .diff_tree_to_tree(
                            compare_to
                                .map(|c| c.tree())
                                .map_or(Ok(None), |v| v.map(Some))
                                .map_err(|err| err.message().to_string())?
                                .as_ref(),
                            Some(&commit.tree().map_err(|err| err.message().to_string())?),
                            None,
                        )
                        .map_err(|err| err.message().to_string())?;

                    let mut diff_err = Ok(());
                    diff.print(DiffFormat::NameStatus, |d, _h, l| {
                        let content = std::str::from_utf8(l.content()).unwrap();
                        let tp = match DiffType::from_str(&content[..1]) {
                            Ok(tp) => tp,
                            Err(err) => {
                                diff_err = Err(err);
                                return false;
                            }
                        };
                        let f = match tp {
                            DiffType::Deleted | DiffType::Modified => d.old_file(),
                            DiffType::Added | DiffType::Renamed => d.new_file(),
                        };
                        diffs.push((
                            f.path().and_then(|p| p.to_str()).unwrap_or("").to_string(),
                            tp,
                        ));
                        true
                    })
                    .map_err(|err| err.message().to_string())?;

                    diff_err?;

                    Some(CommitViewInfo::new(
                        message_fmt,
                        StatefulList::with_items(diffs),
                        info.oid,
                        comp_oid.unwrap_or_else(Oid::zero),
                    ))
                } else {
                    None
                }
        }
        self.file_changed()?;
        Ok(())
    }

    fn file_changed(&mut self) -> Result<(), String> {
        if let (Some(graph), Some(state)) = (&self.graph_state.graph, &self.commit_state.content) {
            self.diff_state.content = if let Some((info, sel_index)) = self
                .graph_state
                .selected
                .and_then(move |sel_idx| graph.commits.get(sel_idx))
                .and_then(|info| {
                    state
                        .diffs
                        .state
                        .selected()
                        .map(|sel_index| (info, sel_index))
                }) {
                let commit = graph
                    .repository
                    .find_commit(info.oid)
                    .map_err(|err| err.message().to_string())?;

                let compare_to = if let Some(sel) = self.graph_state.secondary_selected {
                    let sec_selected_info = graph.commits.get(sel);
                    if let Some(info) = sec_selected_info {
                        Some(
                            graph
                                .repository
                                .find_commit(info.oid)
                                .map_err(|err| err.message().to_string())?,
                        )
                    } else {
                        commit.parent(0).ok()
                    }
                } else {
                    commit.parent(0).ok()
                };
                let comp_oid = compare_to.as_ref().map(|c| c.id());

                let selection = &state.diffs.items[sel_index];

                let mut opts = DiffOptions::new();
                opts.pathspec(&selection.0);
                opts.disable_pathspec_match(true);
                let diff = graph
                    .repository
                    .diff_tree_to_tree(
                        compare_to
                            .map(|c| c.tree())
                            .map_or(Ok(None), |v| v.map(Some))
                            .map_err(|err| err.message().to_string())?
                            .as_ref(),
                        Some(&commit.tree().map_err(|err| err.message().to_string())?),
                        Some(&mut opts),
                    )
                    .map_err(|err| err.message().to_string())?;

                let mut diff_error = Ok(());
                let mut diffs = vec![];

                diff.print(DiffFormat::Patch, |d, h, l| {
                    match print_diff_line(d, h, l) {
                        Ok(line) => diffs.push(line),
                        Err(err) => {
                            diff_error = Err(err.to_string());
                            return false;
                        }
                    }
                    true
                })
                .map_err(|err| err.message().to_string())?;
                diff_error?;

                Some(DiffViewInfo::new(
                    diffs,
                    info.oid,
                    comp_oid.unwrap_or_else(Oid::zero),
                ))
            } else {
                None
            }
        }
        Ok(())
    }
}

fn print_diff_line(
    _delta: DiffDelta,
    _hunk: Option<DiffHunk>,
    line: DiffLine,
) -> Result<String, Error> {
    let mut out = String::new();
    match line.origin() {
        '+' | '-' | ' ' => write!(out, "{}", line.origin())?,
        _ => {}
    }
    write!(out, "{}", std::str::from_utf8(line.content()).unwrap())?;
    Ok(out)
}

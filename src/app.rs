use crate::widgets::branches_view::{BranchItem, BranchItemType};
use crate::widgets::commit_view::{CommitViewInfo, CommitViewState, DiffItem};
use crate::widgets::diff_view::{DiffViewInfo, DiffViewState};
use crate::widgets::graph_view::GraphViewState;
use crate::widgets::list::StatefulList;
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
    Branches,
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
    pub show_branches: bool,
    pub color: bool,
    pub line_numbers: bool,
    pub should_quit: bool,
    pub models_path: PathBuf,
    pub error_message: Option<String>,
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
            show_branches: false,
            color: true,
            line_numbers: true,
            should_quit: false,
            models_path,
            error_message: None,
        }
    }

    pub fn with_graph(mut self, graph: GitGraph, text: Vec<String>, indices: Vec<usize>) -> App {
        let branches = get_branches(&graph);

        self.graph_state.graph = Some(graph);
        self.graph_state.text = text;
        self.graph_state.indices = indices;
        self.graph_state.branches = Some(StatefulList::with_items(branches));

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
            ActiveView::Branches => {
                if let Some(list) = &mut self.graph_state.branches {
                    list.bwd(step);
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
            ActiveView::Branches => {
                if let Some(list) = &mut self.graph_state.branches {
                    list.fwd(step);
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
            ActiveView::Branches => ActiveView::Graph,
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
            ActiveView::Branches => ActiveView::Branches,
            ActiveView::Graph => ActiveView::Branches,
            ActiveView::Commit => ActiveView::Graph,
            ActiveView::Files => ActiveView::Commit,
            ActiveView::Diff => ActiveView::Files,
            ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
            ActiveView::Models => ActiveView::Models,
        }
    }

    pub fn on_enter(&mut self, is_control: bool) -> Result<(), String> {
        match &self.active_view {
            ActiveView::Help(_) => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph)
            }
            ActiveView::Branches => {
                if let Some(graph) = &self.graph_state.graph {
                    if let Some(state) = &self.graph_state.branches {
                        if let Some(sel) = state.state.selected() {
                            let br = &state.items[sel];
                            if let Some(index) = br.index {
                                let branch_info = &graph.all_branches[index];
                                let commit_idx = graph.indices[&branch_info.target];
                                if is_control {
                                    if self.graph_state.selected.is_some() {
                                        self.graph_state.secondary_selected = Some(commit_idx);
                                        self.graph_state.secondary_changed = true;
                                        self.selection_changed()?;
                                    }
                                } else {
                                    self.graph_state.selected = Some(commit_idx);
                                    self.graph_state.secondary_changed = false;
                                    self.selection_changed()?;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn on_backspace(&mut self) -> Result<(), String> {
        match &self.active_view {
            ActiveView::Help(_) | ActiveView::Models => {}
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

    pub fn toggle_branches(&mut self) {
        self.show_branches = !self.show_branches;
    }

    pub fn toggle_line_numbers(&mut self) -> Result<(), String> {
        self.line_numbers = !self.line_numbers;
        self.file_changed()
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

            let models = get_available_models(&self.models_path).map_err(|err| {
                format!(
                    "Unable to load model files from %APP_DATA%/git-graph/models\n{}",
                    err
                )
            })?;
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
                        diffs.push(DiffItem {
                            file: f.path().and_then(|p| p.to_str()).unwrap_or("").to_string(),
                            diff_type: tp,
                        });
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
                opts.pathspec(&selection.file);
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
                    match print_diff_line(&d, &h, &l) {
                        Ok(line) => diffs.push((line, l.old_lineno(), l.new_lineno())),
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

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

fn print_diff_line(
    _delta: &DiffDelta,
    _hunk: &Option<DiffHunk>,
    line: &DiffLine,
) -> Result<String, Error> {
    let mut out = String::new();
    match line.origin() {
        '+' | '-' | ' ' => write!(out, "{}", line.origin())?,
        _ => {}
    }
    write!(out, "{}", std::str::from_utf8(line.content()).unwrap())?;
    Ok(out)
}

fn get_branches(graph: &GitGraph) -> Vec<BranchItem> {
    let mut branches = Vec::new();

    branches.push(BranchItem::new(
        "BRANCHES".to_string(),
        None,
        7,
        BranchItemType::Heading,
    ));
    for idx in &graph.branches {
        let branch = &graph.all_branches[*idx];
        if !branch.is_remote {
            branches.push(BranchItem::new(
                branch.name.clone(),
                Some(*idx),
                branch.visual.term_color,
                BranchItemType::LocalBranch,
            ));
        }
    }

    branches.push(BranchItem::new(
        "REMOTES".to_string(),
        None,
        7,
        BranchItemType::Heading,
    ));
    for idx in &graph.branches {
        let branch = &graph.all_branches[*idx];
        if branch.is_remote {
            branches.push(BranchItem::new(
                branch.name.clone(),
                Some(*idx),
                branch.visual.term_color,
                BranchItemType::RemoteBranch,
            ));
        }
    }

    branches.push(BranchItem::new(
        "TAGS".to_string(),
        None,
        7,
        BranchItemType::Heading,
    ));
    for idx in &graph.tags {
        let branch = &graph.all_branches[*idx];
        branches.push(BranchItem::new(
            branch.name.clone(),
            Some(*idx),
            branch.visual.term_color,
            BranchItemType::Tag,
        ));
    }

    branches
}

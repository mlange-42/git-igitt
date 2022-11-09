use crate::settings::AppSettings;
use crate::util::syntax_highlight::highlight;
use crate::widgets::branches_view::{BranchItem, BranchItemType};
use crate::widgets::commit_view::{CommitViewInfo, CommitViewState, DiffItem};
use crate::widgets::diff_view::{DiffViewInfo, DiffViewState};
use crate::widgets::graph_view::GraphViewState;
use crate::widgets::list::StatefulList;
use crate::widgets::models_view::ModelListState;
use git2::{Commit, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions as GDiffOptions, Oid};
use git_graph::config::get_available_models;
use git_graph::graph::GitGraph;
use git_graph::print::unicode::{format_branches, print_unicode};
use git_graph::settings::Settings;
use std::fmt::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tui::style::Color;

const HASH_COLOR: u8 = 11;

#[derive(PartialEq, Eq)]
pub enum ActiveView {
    Branches,
    Graph,
    Commit,
    Files,
    Diff,
    Models,
    Search,
    Help(u16),
}

pub enum DiffType {
    Added,
    Deleted,
    Modified,
    Renamed,
}

#[derive(PartialEq, Eq)]
pub enum DiffMode {
    Diff,
    Old,
    New,
}

pub struct DiffOptions {
    pub context_lines: u32,
    pub diff_mode: DiffMode,
    pub line_numbers: bool,
    pub syntax_highlight: bool,
    pub wrap_lines: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            context_lines: 3,
            diff_mode: DiffMode::Diff,
            line_numbers: true,
            syntax_highlight: true,
            wrap_lines: false,
        }
    }
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
pub type DiffLines = Vec<(String, Option<u32>, Option<u32>)>;

pub struct App {
    pub settings: AppSettings,
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
    pub models_path: PathBuf,
    pub error_message: Option<String>,
    pub diff_options: DiffOptions,
    pub search_term: Option<String>,
}

impl App {
    pub fn new(
        settings: AppSettings,
        title: String,
        repo_name: String,
        models_path: PathBuf,
    ) -> App {
        App {
            settings,
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
            models_path,
            error_message: None,
            diff_options: DiffOptions::default(),
            search_term: None,
        }
    }

    pub fn with_graph(
        mut self,
        graph: GitGraph,
        graph_lines: Vec<String>,
        text_lines: Vec<String>,
        indices: Vec<usize>,
        select_head: bool,
    ) -> Result<App, String> {
        let branches = get_branches(&graph);

        self.graph_state.graph = Some(graph);
        self.graph_state.graph_lines = graph_lines;
        self.graph_state.text_lines = text_lines;
        self.graph_state.indices = indices;
        self.graph_state.branches = Some(StatefulList::with_items(branches));

        if select_head {
            if let Some(graph) = &self.graph_state.graph {
                if let Some(index) = graph.indices.get(&graph.head.oid) {
                    self.graph_state.selected = Some(*index);
                    self.selection_changed()?;
                }
            }
        }

        Ok(self)
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
        self.graph_state.graph_lines = vec![];
        self.graph_state.text_lines = vec![];
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
            let graph = GitGraph::new(repo, settings, max_commits)?;
            let (graph_lines, text_lines, indices) = print_unicode(&graph, settings)?;

            let sel_idx = sel_oid.and_then(|oid| graph.indices.get(&oid)).cloned();
            let old_idx = self.graph_state.selected;
            self.graph_state.selected = sel_idx;
            if sel_idx.is_some() != old_idx.is_some() {
                self.selection_changed()?;
            }

            self.with_graph(graph, graph_lines, text_lines, indices, false)
        } else {
            Ok(self)
        }
    }

    pub fn on_up(&mut self, is_shift: bool, is_ctrl: bool) -> Result<(bool, bool), String> {
        let step = if is_shift { 10 } else { 1 };
        match self.active_view {
            ActiveView::Graph => {
                if is_ctrl {
                    if self.graph_state.move_secondary_selection(step, false) {
                        if self.graph_state.secondary_selected == self.graph_state.selected {
                            self.graph_state.secondary_selected = None;
                        }
                        return Ok((true, false));
                    }
                } else if self.graph_state.move_selection(step, false) {
                    return Ok((true, false));
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
                    return Ok((false, content.diffs.bwd(step)));
                }
            }
            ActiveView::Diff => {
                if let Some(content) = &mut self.diff_state.content {
                    content.scroll = (
                        content.scroll.0.saturating_sub(step as u16),
                        content.scroll.1,
                    );
                }
            }
            ActiveView::Models => {
                if let Some(state) = &mut self.models_state {
                    state.bwd(step)
                }
            }
            _ => {}
        }
        Ok((false, false))
    }

    pub fn on_down(&mut self, is_shift: bool, is_ctrl: bool) -> Result<(bool, bool), String> {
        let step = if is_shift { 10 } else { 1 };
        match self.active_view {
            ActiveView::Graph => {
                if is_ctrl {
                    if self.graph_state.move_secondary_selection(step, true) {
                        if self.graph_state.secondary_selected == self.graph_state.selected {
                            self.graph_state.secondary_selected = None;
                        }
                        return Ok((true, false));
                    }
                } else if self.graph_state.move_selection(step, true) {
                    return Ok((true, false));
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
                    return Ok((false, content.diffs.fwd(step)));
                }
            }
            ActiveView::Diff => {
                if let Some(content) = &mut self.diff_state.content {
                    content.scroll = (
                        content.scroll.0.saturating_add(step as u16),
                        content.scroll.1,
                    );
                }
            }
            ActiveView::Models => {
                if let Some(state) = &mut self.models_state {
                    state.fwd(step)
                }
            }
            _ => {}
        }
        Ok((false, false))
    }

    pub fn on_home(&mut self) -> Result<bool, String> {
        if let ActiveView::Graph = self.active_view {
            if let Some(graph) = &self.graph_state.graph {
                if let Some(index) = graph.indices.get(&graph.head.oid) {
                    self.graph_state.selected = Some(*index);
                    return Ok(true);
                } else if !self.graph_state.graph_lines.is_empty() {
                    self.graph_state.selected = Some(0);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn on_end(&mut self) -> Result<bool, String> {
        if let ActiveView::Graph = self.active_view {
            if !self.graph_state.indices.is_empty() {
                self.graph_state.selected = Some(self.graph_state.indices.len() - 1);
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn on_right(&mut self, is_shift: bool, is_ctrl: bool) -> Result<bool, String> {
        let mut reload_file_diff = false;
        if is_ctrl {
            let step = if is_shift { 15 } else { 3 };
            match self.active_view {
                ActiveView::Diff => {
                    if let Some(content) = &mut self.diff_state.content {
                        content.scroll = (
                            content.scroll.0,
                            content.scroll.1.saturating_add(step as u16),
                        );
                    }
                }
                ActiveView::Files => {
                    if let Some(content) = &mut self.commit_state.content {
                        content.diffs.state.scroll_x =
                            content.diffs.state.scroll_x.saturating_add(step as u16);
                    }
                }
                ActiveView::Branches => {
                    if let Some(branches) = &mut self.graph_state.branches {
                        branches.state.scroll_x =
                            branches.state.scroll_x.saturating_add(step as u16);
                    }
                }
                _ => {}
            }
        } else {
            self.active_view = match &self.active_view {
                ActiveView::Branches => ActiveView::Graph,
                ActiveView::Graph => ActiveView::Commit,
                ActiveView::Commit => {
                    if let Some(commit) = &mut self.commit_state.content {
                        if commit.diffs.state.selected.is_none() && !commit.diffs.items.is_empty() {
                            commit.diffs.state.selected = Some(0);
                            reload_file_diff = true;
                        }
                    }
                    ActiveView::Files
                }
                ActiveView::Files => ActiveView::Diff,
                ActiveView::Diff => ActiveView::Diff,
                ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
                ActiveView::Models => ActiveView::Models,
                ActiveView::Search => ActiveView::Search,
            }
        }
        Ok(reload_file_diff)
    }
    pub fn on_left(&mut self, is_shift: bool, is_ctrl: bool) {
        if is_ctrl {
            let step = if is_shift { 15 } else { 3 };
            match self.active_view {
                ActiveView::Diff => {
                    if let Some(content) = &mut self.diff_state.content {
                        content.scroll = (
                            content.scroll.0,
                            content.scroll.1.saturating_sub(step as u16),
                        );
                    }
                }
                ActiveView::Files => {
                    if let Some(content) = &mut self.commit_state.content {
                        content.diffs.state.scroll_x =
                            content.diffs.state.scroll_x.saturating_sub(step as u16);
                    }
                }
                ActiveView::Branches => {
                    if let Some(branches) = &mut self.graph_state.branches {
                        branches.state.scroll_x =
                            branches.state.scroll_x.saturating_sub(step as u16);
                    }
                }
                _ => {}
            }
        } else {
            self.active_view = match &self.active_view {
                ActiveView::Branches => ActiveView::Branches,
                ActiveView::Graph => ActiveView::Branches,
                ActiveView::Commit => ActiveView::Graph,
                ActiveView::Files => ActiveView::Commit,
                ActiveView::Diff => ActiveView::Files,
                ActiveView::Help(_) => self.prev_active_view.take().unwrap_or(ActiveView::Graph),
                ActiveView::Models => ActiveView::Models,
                ActiveView::Search => ActiveView::Search,
            }
        }
    }

    pub fn on_enter(&mut self, is_control: bool) -> Result<bool, String> {
        match &self.active_view {
            ActiveView::Help(_) => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph)
            }
            ActiveView::Search => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
                self.search()?;
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
                                        if self.is_fullscreen {
                                            self.active_view = ActiveView::Graph;
                                        }
                                        return Ok(true);
                                    }
                                } else {
                                    self.graph_state.selected = Some(commit_idx);
                                    self.graph_state.secondary_changed = false;
                                    if self.is_fullscreen {
                                        self.active_view = ActiveView::Graph;
                                    }
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    pub fn on_backspace(&mut self) -> Result<bool, String> {
        match &self.active_view {
            ActiveView::Help(_) | ActiveView::Models => {}
            ActiveView::Search => {
                if let Some(term) = &self.search_term {
                    let term = &term[0..(term.len() - 1)];
                    self.search_term = if term.is_empty() {
                        None
                    } else {
                        Some(term.to_string())
                    };
                }
            }
            _ => {
                if self.graph_state.secondary_selected.is_some() {
                    self.graph_state.secondary_selected = None;
                    self.graph_state.secondary_changed = false;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn on_plus(&mut self) -> Result<bool, String> {
        if self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files {
            self.diff_options.context_lines = self.diff_options.context_lines.saturating_add(1);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn on_minus(&mut self) -> Result<bool, String> {
        if self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files {
            self.diff_options.context_lines = self.diff_options.context_lines.saturating_sub(1);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn on_tab(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;
    }

    pub fn on_esc(&mut self) -> Result<bool, String> {
        match self.active_view {
            ActiveView::Models | ActiveView::Help(_) => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
            }
            ActiveView::Search => {
                self.active_view = self.prev_active_view.take().unwrap_or(ActiveView::Graph);
                self.exit_search(true);
            }
            _ => {
                self.active_view = ActiveView::Graph;
                self.is_fullscreen = false;
                if let Some(content) = &mut self.commit_state.content {
                    content.diffs.state.scroll_x = 0;
                }
                self.diff_options.diff_mode = DiffMode::Diff;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn character_entered(&mut self, c: char) {
        if let ActiveView::Search = self.active_view {
            if let Some(term) = &self.search_term {
                self.search_term = Some(format!("{}{}", term, c))
            } else {
                self.search_term = Some(format!("{}", c))
            }
        }
    }

    pub fn open_search(&mut self) {
        // TODO: remove once searching in diffs works
        self.active_view = ActiveView::Graph;

        if let ActiveView::Search = self.active_view {
        } else {
            let mut temp = ActiveView::Search;
            std::mem::swap(&mut temp, &mut self.active_view);
            self.prev_active_view = Some(temp);
        }
    }
    pub fn exit_search(&mut self, _abort: bool) {}

    pub fn search(&mut self) -> Result<bool, String> {
        // TODO: remove once searching in diffs works
        self.active_view = ActiveView::Graph;

        let update = match &self.active_view {
            ActiveView::Branches | ActiveView::Graph | ActiveView::Commit => self.search_graph()?,
            ActiveView::Files | ActiveView::Diff => self.search_diff(),
            _ => false,
        };
        Ok(update)
    }
    fn search_graph(&mut self) -> Result<bool, String> {
        if let Some(search) = &self.search_term {
            let term = search.to_lowercase();

            let search_start = if let Some(sel_idx) = &self.graph_state.selected {
                sel_idx + 1
            } else {
                0
            };
            for idx in search_start..self.graph_state.indices.len() {
                if self.commit_contains(idx, &term) {
                    self.graph_state.selected = Some(idx);
                    return Ok(true);
                }
            }
            for idx in 0..search_start {
                if self.commit_contains(idx, &term) {
                    self.graph_state.selected = Some(idx);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn commit_contains(&self, commit_idx: usize, term: &str) -> bool {
        let num_lines = self.graph_state.text_lines.len();
        let line_start = self.graph_state.indices[commit_idx];
        let line_end = self
            .graph_state
            .indices
            .get(commit_idx + 1)
            .unwrap_or(&num_lines);
        for line_idx in line_start..*line_end {
            if self.graph_state.text_lines[line_idx]
                .to_lowercase()
                .contains(term)
            {
                return true;
            }
        }
        false
    }

    fn search_diff(&mut self) -> bool {
        // TODO implement search in diff panel
        false
    }

    pub fn set_diff_mode(&mut self, mode: DiffMode) -> Result<bool, String> {
        if mode != self.diff_options.diff_mode
            && (self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files)
        {
            self.diff_options.diff_mode = mode;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn toggle_line_numbers(&mut self) -> Result<bool, String> {
        if self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files {
            self.diff_options.line_numbers = !self.diff_options.line_numbers;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn toggle_line_wrap(&mut self) -> Result<bool, String> {
        if self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files {
            self.diff_options.wrap_lines = !self.diff_options.wrap_lines;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn toggle_syntax_highlight(&mut self) -> Result<bool, String> {
        if self.active_view == ActiveView::Diff || self.active_view == ActiveView::Files {
            self.diff_options.syntax_highlight = !self.diff_options.syntax_highlight;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn toggle_layout(&mut self) {
        self.horizontal_split = !self.horizontal_split;
    }

    pub fn toggle_branches(&mut self) {
        self.show_branches = !self.show_branches;
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

    pub fn selection_changed(&mut self) -> Result<(), String> {
        self.reload_diff_message()?;
        let _reload_file = self.reload_diff_files()?;
        Ok(())
    }

    pub fn reload_diff_message(&mut self) -> Result<(), String> {
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
                    let branches = format_branches(graph, info, head, self.color);
                    let message_fmt = crate::util::format::format(&commit, branches, hash_color);

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

                    Some(CommitViewInfo::new(
                        message_fmt,
                        StatefulList::default(),
                        info.oid,
                        comp_oid.unwrap_or_else(Oid::zero),
                    ))
                } else {
                    None
                }
        }
        Ok(())
    }

    pub fn reload_diff_files(&mut self) -> Result<bool, String> {
        if let Some(graph) = &self.graph_state.graph {
            if let Some(content) = &mut self.commit_state.content {
                let commit = graph
                    .repository
                    .find_commit(content.oid)
                    .map_err(|err| err.message().to_string())?;

                let compare_to = if content.compare_oid.is_zero() {
                    None
                } else {
                    Some(
                        graph
                            .repository
                            .find_commit(content.compare_oid)
                            .map_err(|err| err.message().to_string())?,
                    )
                };

                let diffs = get_diff_files(graph, compare_to.as_ref(), &commit)?;

                content.diffs = StatefulList::with_items(diffs)
            }
        }
        Ok(true)
    }

    pub fn clear_file_diff(&mut self) {
        if let Some(content) = &mut self.diff_state.content {
            content.diffs.clear();
            content.highlighted = None;
        }
    }

    pub fn file_changed(&mut self, reset_scroll: bool) -> Result<(), String> {
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

                let diffs = get_file_diffs(
                    graph,
                    compare_to.as_ref(),
                    &commit,
                    &selection.file,
                    &self.diff_options,
                    &self.settings.tab_spaces,
                )?;

                let highlighted = if self.color
                    && self.diff_options.syntax_highlight
                    && self.diff_options.diff_mode != DiffMode::Diff
                    && diffs.len() == 2
                {
                    PathBuf::from(&selection.file)
                        .extension()
                        .and_then(|ext| ext.to_str().and_then(|ext| highlight(&diffs[1].0, ext)))
                } else {
                    None
                };

                let mut info = DiffViewInfo::new(
                    diffs,
                    highlighted,
                    info.oid,
                    comp_oid.unwrap_or_else(Oid::zero),
                );

                if !reset_scroll {
                    if let Some(diff_state) = &self.diff_state.content {
                        info.scroll = diff_state.scroll;
                    }
                }

                Some(info)
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

fn get_diff_files(
    graph: &GitGraph,
    old: Option<&Commit>,
    new: &Commit,
) -> Result<Vec<DiffItem>, String> {
    let mut diffs = vec![];
    let diff = graph
        .repository
        .diff_tree_to_tree(
            old.map(|c| c.tree())
                .map_or(Ok(None), |v| v.map(Some))
                .map_err(|err| err.message().to_string())?
                .as_ref(),
            Some(&new.tree().map_err(|err| err.message().to_string())?),
            None,
        )
        .map_err(|err| err.message().to_string())?;

    let mut diff_err = Ok(());
    diff.print(DiffFormat::NameStatus, |d, _h, l| {
        let content =
            std::str::from_utf8(l.content()).unwrap_or("Invalid UTF8 character in file name.");
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

    Ok(diffs)
}

fn get_file_diffs(
    graph: &GitGraph,
    old: Option<&Commit>,
    new: &Commit,
    path: &str,
    options: &DiffOptions,
    tab_spaces: &str,
) -> Result<DiffLines, String> {
    let mut diffs = vec![];
    let mut opts = GDiffOptions::new();
    opts.context_lines(options.context_lines);
    opts.indent_heuristic(true);
    opts.pathspec(path);
    opts.disable_pathspec_match(true);
    let diff = graph
        .repository
        .diff_tree_to_tree(
            old.map(|c| c.tree())
                .map_or(Ok(None), |v| v.map(Some))
                .map_err(|err| err.message().to_string())?
                .as_ref(),
            Some(&new.tree().map_err(|err| err.message().to_string())?),
            Some(&mut opts),
        )
        .map_err(|err| err.message().to_string())?;

    let mut diff_error = Ok(());

    if options.diff_mode == DiffMode::Diff {
        diff.print(DiffFormat::Patch, |d, h, l| {
            diffs.push((
                print_diff_line(&d, &h, &l).replace('\t', tab_spaces),
                l.old_lineno(),
                l.new_lineno(),
            ));
            true
        })
        .map_err(|err| err.message().to_string())?;
    } else {
        match diff.print(DiffFormat::PatchHeader, |d, _h, l| {
            let (blob_oid, oid) = if options.diff_mode == DiffMode::New {
                (d.new_file().id(), new.id())
            } else {
                (
                    d.old_file().id(),
                    old.map(|c| c.id()).unwrap_or_else(Oid::zero),
                )
            };

            let line = std::str::from_utf8(l.content())
                .unwrap_or("Invalid UTF8 character.")
                .replace('\t', tab_spaces);
            diffs.push((line, None, None));

            if blob_oid.is_zero() {
                diffs.push((
                    format!("File does not exist in {}", &oid.to_string()[..7]),
                    None,
                    None,
                ))
            } else {
                let blob = match graph.repository.find_blob(blob_oid) {
                    Ok(blob) => blob,
                    Err(err) => {
                        diff_error = Err(err.to_string());
                        return false;
                    }
                };

                let text = std::str::from_utf8(blob.content())
                    .map_err(|err| err.to_string())
                    .unwrap_or("Invalid UTF8 character.");
                diffs.push((text.replace('\t', tab_spaces), None, None));
            }
            true
        }) {
            Ok(_) => {}
            Err(_) => {
                let oid = if options.diff_mode == DiffMode::New {
                    new.id()
                } else {
                    old.map(|c| c.id()).unwrap_or_else(Oid::zero)
                };
                diffs.push((
                    format!("File does not exist in {}", &oid.to_string()[..7]),
                    None,
                    None,
                ))
            }
        };
    }
    diff_error?;
    Ok(diffs)
}

fn print_diff_line(_delta: &DiffDelta, _hunk: &Option<DiffHunk>, line: &DiffLine) -> String {
    let mut out = String::new();
    match line.origin() {
        '+' | '-' | ' ' => write!(out, "{}", line.origin()).unwrap(),
        _ => {}
    }
    write!(
        out,
        "{}",
        std::str::from_utf8(line.content()).unwrap_or("Invalid UTF8 character.")
    )
    .unwrap();

    out
}

fn get_branches(graph: &GitGraph) -> Vec<BranchItem> {
    let mut branches = Vec::new();

    branches.push(BranchItem::new(
        "Branches".to_string(),
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
        "Remotes".to_string(),
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
        "Tags".to_string(),
        None,
        7,
        BranchItemType::Heading,
    ));

    let mut tags: Vec<_> = graph
        .tags
        .iter()
        .filter_map(|idx| {
            let branch = &graph.all_branches[*idx];
            if let Ok(commit) = graph.repository.find_commit(branch.target) {
                let time = commit.time();
                Some((
                    BranchItem::new(
                        branch.name.clone(),
                        Some(*idx),
                        branch.visual.term_color,
                        BranchItemType::Tag,
                    ),
                    time.seconds() + time.offset_minutes() as i64 * 60,
                ))
            } else {
                None
            }
        })
        .collect();

    tags.sort_by_key(|bt| -bt.1);

    branches.extend(tags.into_iter().map(|bt| bt.0));

    branches
}

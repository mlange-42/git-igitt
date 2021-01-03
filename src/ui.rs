use crate::app::App;
use crate::widgets::graph_view::GraphView;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders};
use tui::Frame;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    let graph = GraphView::new()
        .block(Block::default().borders(Borders::ALL).title("Graph"))
        .highlight_symbol(">> ");

    f.render_stateful_widget(graph, chunks[0], &mut app.graph_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let commit_block = Block::default().borders(Borders::ALL).title("Commit");
    let diff_block = Block::default().borders(Borders::ALL).title("Diff");

    f.render_widget(commit_block, right_chunks[0]);
    f.render_widget(diff_block, right_chunks[1]);
}

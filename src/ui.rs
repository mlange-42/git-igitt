use crate::app::{ActiveView, App};
use crate::widgets::graph_view::GraphView;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::Frame;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if let ActiveView::Help(scroll) = app.active_view {
        draw_help(f, f.size(), scroll);
        return;
    }

    if app.is_fullscreen {
        match app.active_view {
            ActiveView::Graph => draw_graph(f, f.size(), app),
            ActiveView::Commit => draw_commit(f, f.size(), app),
            ActiveView::Diff => draw_diff(f, f.size(), app),
            ActiveView::Help(_) => {}
        }
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        draw_graph(f, chunks[0], app);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);

        draw_commit(f, right_chunks[0], app);
        draw_diff(f, right_chunks[1], app);
    }
}

fn draw_graph<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title("Graph - H for help");
    if app.active_view == ActiveView::Graph {
        block = block.border_type(BorderType::Thick);
    }

    let graph = GraphView::default().block(block).highlight_symbol(">");

    f.render_stateful_widget(graph, target, &mut app.graph_state);
}

fn draw_commit<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title("Commit");
    if app.active_view == ActiveView::Commit {
        block = block.border_type(BorderType::Thick);
    }

    f.render_widget(block, target);
}

fn draw_diff<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title("Diff");
    if app.active_view == ActiveView::Diff {
        block = block.border_type(BorderType::Thick);
    }

    f.render_widget(block, target);
}

fn draw_help<B: Backend>(f: &mut Frame<B>, target: Rect, scroll: u16) {
    let block = Block::default().borders(Borders::ALL).title("Help");

    let paragraph = Paragraph::new(
        "Q                Quit\n\
         H                Show this help\n\
         Up/Down          Navigate commits\n\
         Shift + Up/Down  Navigate fast\n\
         Home/End         Navigate to first/last\n\
         Left/Right       Change panel\n\
         Tab              Fullscreen panel\n\
         Ecs              Return to default view\n\
         R                Reload repository graph",
    )
    .block(block)
    .scroll((scroll, 0));

    f.render_widget(paragraph, target);
}

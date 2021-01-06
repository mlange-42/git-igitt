use crate::app::{ActiveView, App};
use crate::widgets::commit_view::CommitView;
use crate::widgets::files_view::{FileList, FileListItem};
use crate::widgets::graph_view::GraphView;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::Text;
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
            ActiveView::Files => draw_files(f, f.size(), app),
            ActiveView::Diff => draw_diff(f, f.size(), app),
            ActiveView::Help(_) => {}
        }
    } else {
        let base_split = if app.horizontal_split {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        let sub_split = if app.horizontal_split {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };

        let chunks = Layout::default()
            .direction(base_split)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let right_chunks = Layout::default()
            .direction(sub_split)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);

        match app.active_view {
            ActiveView::Files | ActiveView::Diff => draw_diff(f, chunks[0], app),
            _ => draw_graph(f, chunks[0], app),
        }

        draw_commit(f, right_chunks[0], app);
        draw_files(f, right_chunks[1], app);
    }
}

fn draw_graph<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title("Graph - H for help");
    if app.active_view == ActiveView::Graph {
        block = block.border_type(BorderType::Thick);
    }

    let mut graph = GraphView::default().block(block).highlight_symbol(">", "#");

    if app.color {
        graph = graph.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
    }

    f.render_stateful_widget(graph, target, &mut app.graph_state);
}

fn draw_commit<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title("Commit");
    if app.active_view == ActiveView::Commit {
        block = block.border_type(BorderType::Thick);
    }

    let commit = CommitView::default().block(block).highlight_symbol(">");

    f.render_stateful_widget(commit, target, &mut app.commit_state);
}

fn draw_files<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title("Files");
    if app.active_view == ActiveView::Files {
        block = block.border_type(BorderType::Thick);
    }

    let color = app.color;
    if let Some(state) = &mut app.commit_state.content {
        let items: Vec<_> = state
            .diffs
            .items
            .iter()
            .map(|item| {
                FileListItem::new(if color {
                    Text::styled(
                        format!("{} {}", item.1.to_string(), item.0),
                        Style::default().fg(item.1.to_color()),
                    )
                } else {
                    Text::raw(format!("{} {}", item.1.to_string(), item.0))
                })
            })
            .collect();

        let mut list = FileList::new(items).block(block).highlight_symbol("> ");

        if color {
            list = list.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        }

        f.render_stateful_widget(list, target, &mut state.diffs.state);
    } else {
        f.render_widget(block, target);
    }
}

fn draw_diff<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title("Diff");
    if app.active_view == ActiveView::Diff {
        block = block.border_type(BorderType::Thick);
    }
    let styles = [
        Style::default().fg(Color::LightGreen),
        Style::default().fg(Color::LightRed),
        Style::default().fg(Color::LightBlue),
        Style::default(),
    ];
    if let Some(state) = &app.diff_state.content {
        let scroll = state.scroll;

        let mut text = Text::from("");
        for line in &state.diffs {
            if let Some(pos) = line.find(" @@ ") {
                let (l1, l2) = line.split_at(pos + 3);
                text.extend(style_diff_line(l1, &styles, app.color));
                text.extend(style_diff_line(l2, &styles, app.color));
            } else {
                text.extend(style_diff_line(line, &styles, app.color));
            }
        }

        let paragraph = Paragraph::new(text).block(block).scroll((scroll, 0));

        f.render_widget(paragraph, target);
    } else {
        f.render_widget(block, target);
    }
}

fn style_diff_line<'a>(line: &'a str, styles: &'a [Style; 4], color: bool) -> Text<'a> {
    if !color {
        Text::raw(line)
    } else {
        let style = if line.starts_with('+') {
            styles[0]
        } else if line.starts_with('-') {
            styles[1]
        } else if line.starts_with('@') {
            styles[2]
        } else {
            styles[3]
        };
        Text::styled(line, style)
    }
}

fn draw_help<B: Backend>(f: &mut Frame<B>, target: Rect, scroll: u16) {
    let block = Block::default().borders(Borders::ALL).title("Help");

    let paragraph = Paragraph::new(
        "Q                Quit\n\
         H/F1             Show this help\n\
         \n\
         Up/Down          Select / navigate / scroll\n\
         Shift + Up/Down  Navigate fast\n\
         Home/End         Navigate to first/last\n\
         Ctrl + Up/Down   Secondary selection (compare arbitrary commits)\n\
         Return           Clear secondary selection\n\
         \n\
         Left/Right       Change panel\n\
         Tab              Panel to fullscreen\n\
         Ecs              Return to default view\n\
         L                Toggle horizontal/vertical layout\n\
         \n\
         R                Reload repository graph",
    )
    .block(block)
    .scroll((scroll, 0));

    f.render_widget(paragraph, target);
}

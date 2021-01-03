use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git_graph::graph::GitGraph;
use git_graph::print::format::CommitFormat;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, Settings,
};
use git_igitt::app::App;
use git_igitt::ui;
use std::error::Error;
use std::io::stdout;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, Terminal};

enum Event<I> {
    Input(I),
    Tick,
}

fn main() {
    std::process::exit(match start_app() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}

fn start_app() -> Result<(), Box<dyn Error>> {
    let settings = Settings {
        debug: false,
        colored: true,
        compact: true,
        include_remote: true,
        format: CommitFormat::OneLine,
        wrapping: None,
        characters: Characters::round(),
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::from(BranchSettingsDef::git_flow())?,
        merge_patterns: MergePatterns::default(),
    };

    run(settings)?;

    Ok(())
}

fn run(settings: Settings) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut sout = stdout();
    execute!(sout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(sout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = std::sync::mpsc::channel();

    let tick_rate = Duration::from_millis(250);

    std::thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    let repository = git2::Repository::discover(".").map_err(|err| {
        format!(
            "ERROR: {}\n       Navigate into a repository before running git-igitt.",
            err.message()
        )
    })?;

    let graph = GitGraph::new(repository, &settings, None)?;
    let (lines, indices) = print_unicode(&graph, &settings)?;

    let mut app = App::new("git-igitt", true).with_graph(lines, indices);

    terminal.clear()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }

                KeyCode::Up => app.on_up(),
                KeyCode::Down => app.on_down(),
                _ => {}
            },
            Event::Tick => {}
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

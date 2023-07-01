use clap::{crate_version, Arg, Command};
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::Repository;
use git_graph::{
    config::{create_config, get_available_models, get_model, get_model_name},
    get_repo,
    graph::GitGraph,
    print::{format::CommitFormat, unicode::print_unicode},
    settings::{
        BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, RepoSettings,
        Settings,
    },
};
use git_igitt::app::DiffMode;
use git_igitt::settings::AppSettings;
use git_igitt::{
    app::{ActiveView, App, CurrentBranches},
    dialogs::FileDialog,
    ui,
};
use platform_dirs::AppDirs;
use std::cell::Cell;
use std::time::Instant;
use std::{
    error::Error,
    io::stdout,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use tui::{backend::CrosstermBackend, Terminal};

const REPO_CONFIG_FILE: &str = "git-graph.toml";
const CHECK_CHANGE_RATE: u64 = 2000;
const INITIAL_KEY_REPEAT_TIME: u128 = 100;
const MIN_KEY_REPEAT_TIME: u128 = 50;

enum Event<I> {
    Input(I),
    Update,
}

fn reset_terminal() -> std::result::Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn chain_panic_hook() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic| {
        reset_terminal().unwrap();
        original_hook(panic);
    }));
}

fn main() {
    chain_panic_hook();
    std::process::exit(match from_args() {
        Ok(_) => 0,
        Err(err) => {
            let mut sout = stdout();
            match execute!(sout, LeaveAlternateScreen) {
                Ok(_) => {}
                Err(err) => eprintln!("{}", err),
            }
            eprintln!("{}", err);
            1
        }
    });
}

fn from_args() -> Result<(), String> {
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    create_config(&models_dir)?;

    let app = Command::new("git-igitt")
        .version(crate_version!())
        .about(
            "Interactive Git Terminal app with structured Git graphs.\n    \
                 https://github.com/mlange-42/git-igitt\n\
             \n\
             EXAMPES:\n    \
                 git-graph                   -> Start application\n    \
                 git-graph --style round     -> Start application with a different graph style\n    \
                 git-graph --model <model>   -> Start application using a certain <model>\n    \
                 git-graph model --list      -> List available branching models\n    \
                 git-graph model             -> Show repo's current branching models\n    \
                 git-graph model <model>     -> Permanently set model <model> for this repo",
        )
        .arg(
            Arg::new("path")
                .long("path")
                .short('p')
                .help("Open repository from this path or above. Default '.'")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("max-count")
                .long("max-count")
                .short('n')
                .help("Maximum number of commits")
                .required(false)
                .num_args(1)
                .value_name("n"),
        )
        .arg(
            Arg::new("model")
                .long("model")
                .short('m')
                .help("Branching model. Available presets are [simple|git-flow|none].\n\
                       Default: git-flow. \n\
                       Permanently set the model for a repository with\n\
                         > git-graph model <model>")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("local")
                .long("local")
                .short('l')
                .help("Show only local branches, no remotes.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("sparse")
                .long("sparse")
                .short('S')
                .help("Print a less compact graph: merge lines point to target lines\n\
                       rather than merge commits.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .help("Specify when colors should be used. One of [auto|always|never].\n\
                       Default: auto.")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help("Print without colors. Missing color support should be detected\n\
                       automatically (e.g. when piping to a file).\n\
                       Overrides option '--color'")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("style")
                .long("style")
                .short('s')
                .help("Output style. One of [normal/thin|round|bold|double|ascii].\n  \
                         (First character can be used as abbreviation, e.g. '-s r')")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("tab-width")
                .long("tab-width")
                .help("Tab width for display in diffs. Default: 4.")
                .required(false)
                .num_args(1)
                .value_name("width"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Commit format. One of [oneline|short|medium|full|\"<string>\"].\n  \
                         (First character can be used as abbreviation, e.g. '-f m')\n\
                       Default: oneline.\n\
                       For placeholders supported in \"<string>\", consult 'git-graph --help'")
                .long_help("Commit format. One of [oneline|short|medium|full|\"<string>\"].\n  \
                              (First character can be used as abbreviation, e.g. '-f m')\n\
                            Formatting placeholders for \"<string>\":\n    \
                                %n    newline\n    \
                                %H    commit hash\n    \
                                %h    abbreviated commit hash\n    \
                                %P    parent commit hashes\n    \
                                %p    abbreviated parent commit hashes\n    \
                                %d    refs (branches, tags)\n    \
                                %s    commit summary\n    \
                                %b    commit message body\n    \
                                %B    raw body (subject and body)\n    \
                                %an   author name\n    \
                                %ae   author email\n    \
                                %ad   author date\n    \
                                %as   author date in short format 'YYYY-MM-DD'\n    \
                                %cn   committer name\n    \
                                %ce   committer email\n    \
                                %cd   committer date\n    \
                                %cs   committer date in short format 'YYYY-MM-DD'\n    \
                                \n    \
                                If you add a + (plus sign) after % of a placeholder,\n       \
                                   a line-feed is inserted immediately before the expansion if\n       \
                                   and only if the placeholder expands to a non-empty string.\n    \
                                If you add a - (minus sign) after % of a placeholder, all\n       \
                                   consecutive line-feeds immediately preceding the expansion are\n       \
                                   deleted if and only if the placeholder expands to an empty string.\n    \
                                If you add a ' ' (space) after % of a placeholder, a space is\n       \
                                   inserted immediately before the expansion if and only if\n       \
                                   the placeholder expands to a non-empty string.\n\
                            \n    \
                                See also the respective git help: https://git-scm.com/docs/pretty-formats\n")
                .required(false)
                .num_args(1),
        )
        .subcommand(Command::new("model")
            .about("Prints or permanently sets the branching model for a repository.")
            .arg(
                Arg::new("model")
                    .help("The branching model to be used. Available presets are [simple|git-flow|none].\n\
                           When not given, prints the currently set model.")
                    .value_name("model")
                    .num_args(1)
                    .required(false)
                    .index(1))
            .arg(
                Arg::new("list")
                    .long("list")
                    .short('l')
                    .help("List all available branching models.")
                    .required(false)
                    .num_args(0),
            ));

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("model") {
        if matches.get_flag("list") {
            println!(
                "{}",
                itertools::join(get_available_models(&models_dir)?, "\n")
            );
            return Ok(());
        }
    }

    let dot = ".".to_string();
    let path = matches.get_one::<String>("path").unwrap_or(&dot);

    let repository = get_repo(path);

    if let Some(matches) = matches.subcommand_matches("model") {
        match repository {
            Ok(repository) => {
                match matches.get_one::<String>("model") {
                    None => {
                        let curr_model = get_model_name(&repository, REPO_CONFIG_FILE)?;
                        match curr_model {
                            None => print!("No branching model set"),
                            Some(model) => print!("{}", model),
                        }
                    }
                    Some(model) => set_model(&repository, model, REPO_CONFIG_FILE, &models_dir)?,
                };
                return Ok(());
            }
            Err(err) => return Err(format!("ERROR: {}\n       Navigate into a repository before running git-graph, or use option --path", err.message())),
        }
    }

    let commit_limit = match matches.get_one::<String>("max-count") {
        None => None,
        Some(str) => match str.parse::<usize>() {
            Ok(val) => Some(val),
            Err(_) => {
                return Err(format![
                    "Option max-count must be a positive number, but got '{}'",
                    str
                ])
            }
        },
    };
    let tab_width = match matches.get_one::<String>("tab-width") {
        None => None,
        Some(str) => match str.parse::<usize>() {
            Ok(val) => Some(val),
            Err(_) => {
                return Err(format![
                    "Option tab-width must be a positive number, but got '{}'",
                    str
                ])
            }
        },
    };

    let include_remote = !matches.get_flag("local");

    let compact = !matches.get_flag("sparse");
    let style = matches
        .get_one::<String>("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::round()))?;

    let model = matches.get_one::<String>("model");

    let format = match matches.get_one::<String>("format") {
        None => CommitFormat::OneLine,
        Some(str) => CommitFormat::from_str(str)?,
    };

    let colored = if matches.get_flag("no-color") {
        false
    } else if let Some(mode) = matches.get_one::<String>("color") {
        match mode.as_str() {
            "auto" => !cfg!(windows) || yansi::Paint::enable_windows_ascii(),
            "always" => {
                if cfg!(windows) {
                    yansi::Paint::enable_windows_ascii();
                }
                true
            }
            "never" => false,
            other => {
                return Err(format!(
                    "Unknown color mode '{}'. Supports [auto|always|never].",
                    other
                ))
            }
        }
    } else {
        !cfg!(windows) || yansi::Paint::enable_windows_ascii()
    };

    let app_settings = AppSettings::default().tab_width(tab_width.unwrap_or(4));

    let settings = Settings {
        debug: false,
        colored,
        compact,
        include_remote,
        format,
        wrapping: None,
        characters: style,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::from(BranchSettingsDef::none()).map_err(|err| err.to_string())?,
        merge_patterns: MergePatterns::default(),
    };

    run(
        repository.ok(),
        settings,
        app_settings,
        model.map(|x| &**x),
        commit_limit,
    )
    .map_err(|err| err.to_string())?;

    Ok(())
}

fn run(
    mut repository: Option<Repository>,
    mut settings: Settings,
    app_settings: AppSettings,
    model: Option<&str>,
    max_commits: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut sout = stdout();
    execute!(sout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(sout);
    let mut terminal = Terminal::new(backend)?;

    let repo_refresh_interval = Duration::from_millis(CHECK_CHANGE_RATE);

    let mut file_dialog =
        FileDialog::new("Open repository", settings.colored).map_err(|err| err.to_string())?;
    if let Some(repo) = &repository {
        if repo.is_shallow() {
            file_dialog.set_error(format!("{} is a shallow clone. Shallow clones are not supported due to a missing feature in the underlying libgit2 library.", repo.path().parent().unwrap().display()));
            let selected = repo.path().parent().unwrap();
            //file_dialog.selection = Some(PathBuf::from(selected));
            file_dialog.location = PathBuf::from(selected.parent().unwrap());
            file_dialog.selection_changed(Some(PathBuf::from(selected)))?;
        } else {
            file_dialog.selection_changed(None)?;
        }
    } else {
        file_dialog.selection_changed(None)?;
    }

    let mut app = if let Some(repository) = repository.take() {
        if repository.is_shallow() {
            None
        } else {
            Some(create_app(
                repository,
                &mut settings,
                &app_settings,
                model,
                max_commits,
            )?)
        }
    } else {
        None
    };

    let next_repo_refresh = &Cell::new(Instant::now() + repo_refresh_interval);
    let next_diff_update: &Cell<Option<Instant>> = &Cell::new(None);
    let next_file_update: &Cell<Option<Instant>> = &Cell::new(None);
    let mut reset_diff_scroll = false;

    let mut next_event = {
        let mut sx_old = 0;
        let mut sy_old = 0;

        move || loop {
            let mut next_event_time = next_repo_refresh.get();
            if let Some(next) = next_diff_update.get() {
                next_event_time = next.min(next_event_time)
            }
            if let Some(next) = next_file_update.get() {
                next_event_time = next.min(next_event_time)
            }

            let timeout = next_event_time.saturating_duration_since(Instant::now());

            if event::poll(timeout).unwrap() {
                match event::read().unwrap() {
                    CEvent::Key(key) => return Event::Input(key),
                    CEvent::Mouse(_) => (),
                    CEvent::Resize(sx, sy) => {
                        if sx != sx_old || sy != sy_old {
                            sx_old = sx;
                            sy_old = sy;
                            return Event::Update;
                        }
                    }
                    _ => {}
                }
                continue;
            }
            return Event::Update;
        }
    };

    terminal.clear()?;

    let mut last_key_time = Instant::now();
    let mut last_key = KeyCode::Esc;
    let mut key_repeat_time = INITIAL_KEY_REPEAT_TIME / 2;

    loop {
        app = if let Some(mut app) = app.take() {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            let mut open_file = false;
            if app.error_message.is_some() {
                if let Event::Input(event) = next_event() {
                    match event.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.clear_error();
                        }
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            let mut reload_diffs = false;
            let mut reload_file = false;
            let mut reset_scroll = true;
            if app.active_view == ActiveView::Search {
                if let Event::Input(event) = next_event() {
                    match event.code {
                        KeyCode::Char(c) => app.character_entered(c),
                        KeyCode::Esc => reload_file = app.on_esc()?,
                        KeyCode::Enter | KeyCode::F(3) => {
                            reload_diffs =
                                app.on_enter(event.modifiers.contains(KeyModifiers::CONTROL))?
                        }
                        KeyCode::Backspace => reload_diffs = app.on_backspace()?,
                        _ => {}
                    }
                }
            } else {
                match next_event() {
                    Event::Input(event) => {
                        let now = Instant::now();
                        if event.code == last_key {
                            let duration =
                                (now.saturating_duration_since(last_key_time)).as_millis();
                            if duration < key_repeat_time && 2 * duration > MIN_KEY_REPEAT_TIME {
                                key_repeat_time = duration;
                            }
                        } else {
                            last_key = event.code;
                        }
                        last_key_time = now;

                        match event.code {
                            KeyCode::Char('q') => {
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                terminal.show_cursor()?;
                                break;
                            }
                            KeyCode::Char('s') => {
                                reload_file = app.toggle_syntax_highlight()?;
                                reset_scroll = false
                            }
                            KeyCode::Char('h') => app.show_help(),
                            KeyCode::F(1) => app.show_help(),
                            KeyCode::Char('m') => match app.active_view {
                                ActiveView::Models | ActiveView::Search | ActiveView::Help(_) => {}
                                _ => {
                                    if let Err(err) = app.select_model() {
                                        app.set_error(err);
                                    }
                                }
                            },
                            KeyCode::Char('r') => app = app.reload(&settings, max_commits)?,
                            KeyCode::Char('l') => {
                                if event.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.toggle_line_numbers()?;
                                } else {
                                    app.toggle_layout();
                                }
                            }
                            KeyCode::Char('w') => {
                                if event.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.toggle_line_wrap()?;
                                }
                            }
                            KeyCode::Char('b') => app.toggle_branches(),
                            KeyCode::Char('o') => match app.active_view {
                                ActiveView::Models | ActiveView::Search | ActiveView::Help(_) => {}
                                _ => {
                                    if event.modifiers.contains(KeyModifiers::CONTROL) {
                                        if let Some(graph) = &app.graph_state.graph {
                                            let path = graph.repository.path();
                                            let path = path.parent().unwrap_or(path);
                                            file_dialog.location =
                                                PathBuf::from(path.parent().unwrap_or(path));
                                            file_dialog.selection = Some(PathBuf::from(path));
                                        } else {
                                            file_dialog.location = std::env::current_dir()?;
                                            file_dialog.selection = None
                                        }
                                        open_file = true;
                                    } else {
                                        let reset = app.diff_options.diff_mode == DiffMode::Diff;
                                        reload_file = app.set_diff_mode(DiffMode::Old)?;
                                        reset_scroll = reset;
                                    }
                                }
                            },
                            KeyCode::Char('f')
                                if event.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                match app.active_view {
                                    ActiveView::Models
                                    | ActiveView::Search
                                    | ActiveView::Help(_) => {}
                                    _ => app.open_search(),
                                }
                            }
                            KeyCode::F(3) => match app.active_view {
                                ActiveView::Models | ActiveView::Search | ActiveView::Help(_) => {}
                                _ => {
                                    if app.search_term.is_none() {
                                        app.open_search();
                                    } else {
                                        reload_diffs = app.search()?
                                    }
                                }
                            },
                            KeyCode::Char('n') => {
                                let reset = app.diff_options.diff_mode == DiffMode::Diff;
                                reload_file = app.set_diff_mode(DiffMode::New)?;
                                reset_scroll = reset;
                            }
                            KeyCode::Char('d') => {
                                reload_file = app.set_diff_mode(DiffMode::Diff)?;
                            }
                            KeyCode::Char('p') => {
                                if app.active_view == ActiveView::Models {
                                    let (a, s, result) =
                                        set_app_model(app, settings, max_commits, true)?;
                                    app = a;
                                    settings = s;
                                    if let Err(err) = result {
                                        app.set_error(err);
                                        app.active_view = ActiveView::Graph;
                                    }
                                }
                            }

                            KeyCode::Char('+') => {
                                reload_file = app.on_plus()?;
                                reset_scroll = false;
                            }
                            KeyCode::Char('-') => {
                                reload_file = app.on_minus()?;
                                reset_scroll = false;
                            }

                            KeyCode::Up => {
                                let (rd, rf) = app.on_up(
                                    event.modifiers.contains(KeyModifiers::SHIFT),
                                    event.modifiers.contains(KeyModifiers::CONTROL),
                                )?;
                                reload_diffs = rd;
                                reload_file = rf;
                            }
                            KeyCode::Down => {
                                let (rd, rf) = app.on_down(
                                    event.modifiers.contains(KeyModifiers::SHIFT),
                                    event.modifiers.contains(KeyModifiers::CONTROL),
                                )?;
                                reload_diffs = rd;
                                reload_file = rf;
                            }
                            KeyCode::Home => reload_diffs = app.on_home()?,
                            KeyCode::End => reload_diffs = app.on_end()?,
                            KeyCode::Left => app.on_left(
                                event.modifiers.contains(KeyModifiers::SHIFT),
                                event.modifiers.contains(KeyModifiers::CONTROL),
                            ),
                            KeyCode::Right => {
                                reload_file = app.on_right(
                                    event.modifiers.contains(KeyModifiers::SHIFT),
                                    event.modifiers.contains(KeyModifiers::CONTROL),
                                )?
                            }
                            KeyCode::Tab => app.on_tab(),
                            KeyCode::Esc => reload_file = app.on_esc()?,
                            KeyCode::Enter => {
                                if app.active_view == ActiveView::Models {
                                    let (a, s, result) =
                                        set_app_model(app, settings, max_commits, true)?;
                                    app = a;
                                    settings = s;
                                    if let Err(err) = result {
                                        app.set_error(err);
                                        app.active_view = ActiveView::Graph;
                                    }
                                } else {
                                    reload_diffs = app
                                        .on_enter(event.modifiers.contains(KeyModifiers::CONTROL))?
                                }
                            }
                            KeyCode::Backspace => {
                                if app.active_view != ActiveView::Models {
                                    reload_diffs = app.on_backspace()?
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::Update => {
                        let now = Instant::now();
                        if next_repo_refresh.get() <= now {
                            if app.graph_state.graph.is_some() && has_changed(&mut app)? {
                                app = app.reload(&settings, max_commits)?;
                            }
                            next_repo_refresh.set(now + repo_refresh_interval);
                        }
                        if let Some(next) = next_diff_update.get() {
                            if next <= now {
                                reload_file = app.reload_diff_files()?;
                                next_diff_update.set(None);
                            }
                        }
                        if let Some(next) = next_file_update.get() {
                            if next <= now {
                                app.file_changed(reset_diff_scroll)?;
                                next_file_update.set(None);
                            }
                        }
                    }
                }
            };
            if reload_diffs {
                app.reload_diff_message()?;
                next_diff_update.set(Some(
                    Instant::now() + Duration::from_millis(2 * key_repeat_time as u64),
                ));
            }
            if reload_file {
                if reset_scroll {
                    app.clear_file_diff();
                }
                reset_diff_scroll = reset_scroll;
                next_file_update.set(Some(
                    Instant::now() + Duration::from_millis(2 * key_repeat_time as u64),
                ));
            }

            if open_file {
                let prev = if let Some(graph) = &app.graph_state.graph {
                    graph.repository.path().parent().map(PathBuf::from)
                } else {
                    None
                };
                file_dialog.previous_app = Some(app);
                file_dialog.selection_changed(prev)?;
                None
            } else {
                Some(app)
            }
        } else {
            terminal.draw(|f| ui::draw_open_repo(f, &mut file_dialog))?;

            let mut app = None;
            if file_dialog.error_message.is_some() {
                if let Event::Input(event) = next_event() {
                    match event.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            file_dialog.clear_error();
                        }
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        _ => {}
                    }
                }
            } else if let Event::Input(event) = next_event() {
                match event.code {
                    KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;
                        break;
                    }
                    KeyCode::Char('o') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(prev_app) = file_dialog.previous_app.take() {
                            app = Some(prev_app);
                        } else {
                            file_dialog.set_error("No repository to return to.\nSelect a Git rrpository or quit with Q.".to_string())
                        }
                    }
                    KeyCode::Esc => {
                        if let Some(prev_app) = file_dialog.previous_app.take() {
                            app = Some(prev_app);
                        } else {
                            file_dialog.set_error("No repository to return to.\nSelect a Git rrpository or quit with Q.".to_string())
                        }
                    }
                    KeyCode::Up => file_dialog.on_up(event.modifiers.contains(KeyModifiers::SHIFT)),
                    KeyCode::Down => {
                        file_dialog.on_down(event.modifiers.contains(KeyModifiers::SHIFT))
                    }
                    KeyCode::Left => file_dialog.on_left()?,
                    KeyCode::Right => file_dialog.on_right()?,
                    KeyCode::Enter => {
                        file_dialog.on_enter();
                        if let Some(path) = &file_dialog.selection {
                            match get_repo(path) {
                                Ok(repo) => {
                                    if repo.is_shallow() {
                                        file_dialog.set_error(format!("{} is a shallow clone. Shallow clones are not supported due to a missing feature in the underlying libgit2 library.", repo.path().parent().unwrap().display()));
                                    } else {
                                        app = Some(create_app(
                                            repo,
                                            &mut settings,
                                            &app_settings,
                                            model,
                                            max_commits,
                                        )?)
                                    }
                                }
                                Err(_) => {
                                    file_dialog.on_right()?;
                                }
                            };
                        }
                    }
                    _ => {}
                };
            }
            app
        };
    }

    Ok(())
}

fn set_app_model(
    mut app: App,
    mut settings: Settings,
    max_commits: Option<usize>,
    permanent: bool,
) -> Result<(App, Settings, Result<(), String>), String> {
    if let (Some(state), Some(graph)) = (&app.models_state, &app.graph_state.graph) {
        if let Some(sel) = state.state.selected() {
            let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
            let mut models_dir = app_dir;
            models_dir.push("models");

            let model = &state.models[sel][..];
            let temp_model = model.to_string();

            let the_model = match get_model(
                &graph.repository,
                Some(model),
                REPO_CONFIG_FILE,
                &models_dir,
            ) {
                Ok(model) => model,
                Err(err) => {
                    return Ok((
                        app,
                        settings,
                        Err(format!("Unable to load model '{}'.\n{}", temp_model, err)),
                    ))
                }
            };

            if permanent {
                if let Err(err) = set_model(&graph.repository, model, REPO_CONFIG_FILE, &models_dir)
                {
                    return Ok((app, settings, Err(err)));
                }
            }

            app.on_esc()?;

            settings.branches = match BranchSettings::from(the_model) {
                Ok(branch_def) => branch_def,
                Err(err) => {
                    return Ok((
                        app,
                        settings,
                        Err(format!("Unable to parse model '{}'.\n{}", temp_model, err)),
                    ))
                }
            };
            app = app.reload(&settings, max_commits)?;
        }
    }
    Ok((app, settings, Ok(())))
}

/// Permanently sets the branching model for a repository
pub fn set_model<P: AsRef<Path>>(
    repository: &Repository,
    model: &str,
    repo_config_file: &str,
    app_model_path: &P,
) -> Result<(), String> {
    let models = get_available_models(&app_model_path)?;

    if !models.contains(&model.to_string()) {
        return Err(format!(
            "ERROR: No branching model named '{}' found in {}\n       Available models are: {}",
            model,
            app_model_path.as_ref().display(),
            itertools::join(models, ", ")
        ));
    }

    let mut config_path = PathBuf::from(repository.path());
    config_path.push(repo_config_file);

    let config = RepoSettings {
        model: model.to_string(),
    };

    let str = toml::to_string_pretty(&config).map_err(|err| err.to_string())?;
    std::fs::write(&config_path, str).map_err(|err| {
        format!(
            "Can't write repository settings to file {}\n{}",
            &config_path.display(),
            err
        )
    })?;

    Ok(())
}

fn create_app(
    repository: Repository,
    settings: &mut Settings,
    app_settings: &AppSettings,
    model: Option<&str>,
    max_commits: Option<usize>,
) -> Result<App, String> {
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    let the_model = get_model(&repository, model, REPO_CONFIG_FILE, &models_dir)?;
    settings.branches = BranchSettings::from(the_model).map_err(|err| err.to_string())?;

    let name = &repository
        .path()
        .parent()
        .and_then(|p| p.components().last().and_then(|c| c.as_os_str().to_str()))
        .unwrap_or("unknown")
        .to_string();

    let graph = GitGraph::new(repository, settings, max_commits)?;
    let branches = get_branches(&graph)?;
    let (graph_lines, text_lines, indices) = print_unicode(&graph, settings)?;

    Ok(App::new(
        app_settings.clone(),
        format!("git-igitt - {}", name),
        name.clone(),
        models_dir,
    )
    .with_graph(graph, graph_lines, text_lines, indices, true)?
    .with_branches(branches)
    .with_color(settings.colored))
}

fn has_changed(app: &mut App) -> Result<bool, String> {
    if let Some(graph) = &app.graph_state.graph {
        let branches = get_branches(graph)?;

        if app.curr_branches != branches {
            app.curr_branches = branches;
            return Ok(true);
        }

        let head = graph
            .repository
            .head()
            .map_err(|err| err.message().to_string())?;

        let name = head.name().ok_or_else(|| "No name for HEAD".to_string())?;
        let name = if name == "HEAD" { name } else { &name[11..] };
        if graph.head.name != name
            || graph.head.oid != head.target().ok_or_else(|| "No id for HEAD".to_string())?
            || graph.head.is_branch != head.is_branch()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn get_branches(graph: &GitGraph) -> Result<CurrentBranches, String> {
    graph
        .repository
        .branches(None)
        .map_err(|err| err.message().to_string())?
        .map(|br| {
            br.and_then(|(br, _tp)| {
                br.name()
                    .map(|n| (n.map(|n| n.to_string()), br.get().target()))
            })
        })
        .collect::<Result<CurrentBranches, _>>()
        .map_err(|err| err.message().to_string())
}

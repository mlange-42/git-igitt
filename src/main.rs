use clap::{crate_version, Arg, SubCommand};
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
use git_igitt::{
    app::{ActiveView, App, CurrentBranches},
    dialogs::FileDialog,
    ui,
};
use platform_dirs::AppDirs;
use std::{
    error::Error,
    io::stdout,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};

const REPO_CONFIG_FILE: &str = "git-graph.toml";
const TICK_RATE: u64 = 2000;
const CHECK_CHANGE_RATE: u64 = 2000;

enum Event<I> {
    Input(I),
    Tick,
    Update,
}

fn main() {
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

    let app = clap::App::new("git-igitt")
        .version(crate_version!())
        .about(
            "Interactive Git Terminal app with structured Git graphs.\n    \
                 https://github.com/mlange-42/git-igitt\n\
             \n\
             EXAMPES:\n    \
                 git-graph                   -> Show graph\n    \
                 git-graph --style round     -> Show graph in a different style\n    \
                 git-graph --model <model>   -> Show graph using a certain <model>\n    \
                 git-graph model --list      -> List available branching models\n    \
                 git-graph model             -> Show repo's current branching models\n    \
                 git-graph model <model>     -> Permanently set model <model> for this repo",
        )
        .arg(
            Arg::with_name("path")
                .long("path")
                .short("p")
                .help("Open repository from this path or above. Default '.'")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-count")
                .long("max-count")
                .short("n")
                .help("Maximum number of commits")
                .required(false)
                .takes_value(true)
                .value_name("n"),
        )
        .arg(
            Arg::with_name("model")
                .long("model")
                .short("m")
                .help("Branching model. Available presets are [simple|git-flow|none].\n\
                       Default: git-flow. \n\
                       Permanently set the model for a repository with\n\
                         > git-graph model <model>")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("local")
                .long("local")
                .short("l")
                .help("Show only local branches, no remotes.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("sparse")
                .long("sparse")
                .short("S")
                .help("Print a less compact graph: merge lines point to target lines\n\
                       rather than merge commits.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .help("Specify when colors should be used. One of [auto|always|never].\n\
                       Default: auto.")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-color")
                .long("no-color")
                .help("Print without colors. Missing color support should be detected\n\
                       automatically (e.g. when piping to a file).\n\
                       Overrides option '--color'")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("style")
                .long("style")
                .short("s")
                .help("Output style. One of [normal/thin|round|bold|double|ascii].\n  \
                         (First character can be used as abbreviation, e.g. '-s r')")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("format")
                .long("format")
                .short("f")
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
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("model")
            .about("Prints or permanently sets the branching model for a repository.")
            .arg(
                Arg::with_name("model")
                    .help("The branching model to be used. Available presets are [simple|git-flow|none].\n\
                           When not given, prints the currently set model.")
                    .value_name("model")
                    .takes_value(true)
                    .required(false)
                    .index(1))
            .arg(
                Arg::with_name("list")
                    .long("list")
                    .short("l")
                    .help("List all available branching models.")
                    .required(false)
                    .takes_value(false),
            ));

    let matches = app.clone().get_matches();

    if let Some(matches) = matches.subcommand_matches("model") {
        if matches.is_present("list") {
            println!(
                "{}",
                itertools::join(get_available_models(&models_dir)?, "\n")
            );
            return Ok(());
        }
    }

    let path = matches.value_of("path").unwrap_or(".");

    let repository = get_repo(path);

    if let Some(matches) = matches.subcommand_matches("model") {
        match repository {
            Ok(repository) => {
                match matches.value_of("model") {
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

    let commit_limit = match matches.value_of("max-count") {
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

    let include_remote = !matches.is_present("local");

    let compact = !matches.is_present("sparse");
    let style = matches
        .value_of("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::round()))?;

    let model = matches.value_of("model");

    let format = match matches.value_of("format") {
        None => CommitFormat::OneLine,
        Some(str) => CommitFormat::from_str(str)?,
    };

    let colored = if matches.is_present("no-color") {
        false
    } else if let Some(mode) = matches.value_of("color") {
        match mode {
            "auto" => (!cfg!(windows) || yansi::Paint::enable_windows_ascii()),
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

    run(repository.ok(), settings, model, commit_limit).map_err(|err| err.to_string())?;

    Ok(())
}

fn run(
    mut repository: Option<Repository>,
    mut settings: Settings,
    model: Option<&str>,
    max_commits: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut sout = stdout();
    execute!(sout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(sout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = std::sync::mpsc::channel();

    let tick_rate = Duration::from_millis(TICK_RATE);
    let update_tick_rate = Duration::from_millis(CHECK_CHANGE_RATE);

    let mut app = if let Some(repository) = repository.take() {
        Some(create_app(repository, &mut settings, model, max_commits)?)
    } else {
        None
    };

    let mut file_dialog =
        FileDialog::new("Open repository", settings.colored).map_err(|err| err.to_string())?;
    file_dialog.selection_changed(None)?;

    std::thread::spawn(move || {
        let mut last_update = Instant::now();
        let mut sx_old = 0;
        let mut sy_old = 0;
        loop {
            let timeout = tick_rate;
            if event::poll(timeout).unwrap() {
                match event::read().unwrap() {
                    CEvent::Key(key) => tx.send(Event::Input(key)).expect("Can't send key event"),
                    CEvent::Mouse(_) => {}
                    CEvent::Resize(sx, sy) => {
                        if sx != sx_old || sy != sy_old {
                            sx_old = sx;
                            sy_old = sy;
                            tx.send(Event::Tick).expect("Can't send resize event")
                        }
                    }
                }
            }
            if last_update.elapsed() >= update_tick_rate {
                tx.send(Event::Update).unwrap();
                last_update = Instant::now();
            }
        }
    });

    terminal.clear()?;

    loop {
        app = if let Some(mut app) = app.take() {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            let mut open_file = false;

            if app.error_message.is_some() {
                if let Event::Input(event) = rx.recv()? {
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
            } else {
                match rx.recv()? {
                    Event::Input(event) => match event.code {
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        KeyCode::Char('h') | KeyCode::F(1) => {
                            app.show_help();
                        }
                        KeyCode::Char('m') => {
                            if let Err(err) = app.select_model() {
                                app.set_error(err);
                            }
                        }
                        KeyCode::Char('r') => {
                            app = app.reload(&settings, max_commits)?;
                        }
                        KeyCode::Char('l') => {
                            if event.modifiers.contains(KeyModifiers::CONTROL) {
                                app.toggle_line_numbers()?;
                            } else {
                                app.toggle_layout();
                            }
                        }
                        KeyCode::Char('b') => {
                            app.toggle_branches();
                        }
                        KeyCode::Char('o') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let Some(graph) = &app.graph_state.graph {
                                let path = graph.repository.path();
                                let path = path.parent().unwrap_or(path);
                                file_dialog.location = PathBuf::from(path.parent().unwrap_or(path));
                                file_dialog.selection = Some(PathBuf::from(path));
                            } else {
                                file_dialog.location = std::env::current_dir()?;
                                file_dialog.selection = None
                            }
                            open_file = true;
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

                        KeyCode::Up => app.on_up(
                            event.modifiers.contains(KeyModifiers::SHIFT),
                            event.modifiers.contains(KeyModifiers::CONTROL),
                        )?,
                        KeyCode::Down => app.on_down(
                            event.modifiers.contains(KeyModifiers::SHIFT),
                            event.modifiers.contains(KeyModifiers::CONTROL),
                        )?,
                        KeyCode::Home => app.on_home()?,
                        KeyCode::End => app.on_end()?,
                        KeyCode::Left => app.on_left(),
                        KeyCode::Right => app.on_right(),
                        KeyCode::Tab => app.on_tab(),
                        KeyCode::Esc => app.on_esc(),
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
                                app.on_enter()?
                            }
                        }
                        _ => {}
                    },
                    Event::Update => {
                        if app.graph_state.graph.is_some() && has_changed(&mut app)? {
                            app = app.reload(&settings, max_commits)?;
                        }
                    }
                    Event::Tick => {}
                }
            }
            if app.should_quit {
                break;
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
                if let Event::Input(event) = rx.recv()? {
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
            } else if let Event::Input(event) = rx.recv()? {
                match event.code {
                    KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;
                        break;
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
                                    app = Some(create_app(repo, &mut settings, model, max_commits)?)
                                }
                                Err(err) => {
                                    file_dialog.error_message = Some(format!(
                                        "Can't open repository at {}\n{}",
                                        path.display(),
                                        err.message().to_string()
                                    ));
                                }
                            };
                        }
                    }
                    _ => {}
                };
            }
            app
        }
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

            app.on_esc();

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
            err.to_string()
        )
    })?;

    Ok(())
}

fn create_app(
    repository: Repository,
    settings: &mut Settings,
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

    let graph = GitGraph::new(repository, &settings, max_commits)?;
    let branches = get_branches(&graph)?;
    let (lines, indices) = print_unicode(&graph, &settings)?;

    Ok(
        App::new(format!("git-igitt - {}", name), name.clone(), models_dir)
            .with_graph(graph, lines, indices)
            .with_branches(branches)
            .with_color(settings.colored),
    )
}

fn has_changed(app: &mut App) -> Result<bool, String> {
    if let Some(graph) = &app.graph_state.graph {
        let branches = get_branches(&graph)?;

        if app.curr_branches != branches {
            app.curr_branches = branches;
            return Ok(true);
        }

        let head = graph
            .repository
            .head()
            .map_err(|err| err.message().to_string())?;

        let name = head.name().ok_or_else(|| "No name for HEAD".to_string())?;
        let name = if name == "HEAD" { &name } else { &name[11..] };
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

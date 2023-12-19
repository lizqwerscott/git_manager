use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::path::Path;
use tokio::sync::mpsc;

mod components;
mod gitrepo;
mod states;
pub mod utils;

use gitrepo::search_all_git_repo;
use gitrepo::GitRepo;
use states::{AppAction, AppMode};
use utils::BDEResult;

use components::{input::Input, reposhow::ReposShow, Component};

#[derive(Debug)]
struct App {
    repos: Vec<GitRepo>,
    runp: bool,

    run_mode: AppMode,

    component_input: Input,
    component_repos_show: ReposShow,
}

impl App {
    fn handle_events(&mut self) -> BDEResult<Option<AppAction>> {
        if !event::poll(std::time::Duration::from_millis(50))? {
            return Ok(None);
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(match self.run_mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Char('q') => Some(AppAction::Quit),
                        _ => self.component_repos_show.handle_events(key)?,
                    },
                    AppMode::Editing => self.component_input.handle_events(key)?,
                });
            }
        }

        Ok(None)
    }

    fn ui(&mut self, f: &mut Frame) -> BDEResult<()> {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(f.size());

        let (msg, style) = match self.run_mode {
            AppMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "f".bold(),
                    " to start filter repo, ".bold(),
                    "g".into(),
                    " to refresh repo.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            AppMode::Editing => (
                vec!["Press ".into(), "Esc".bold(), " to stop search, ".into()],
                Style::default(),
            ),
        };

        let mut text = Text::from(Line::from(msg));
        text.patch_style(style);
        f.render_widget(Paragraph::new(text), main_layout[0]);

        self.component_input
            .draw(self.run_mode, f, main_layout[1])?;
        self.component_repos_show
            .draw(self.run_mode, f, main_layout[2])?;

        Ok(())
    }

    async fn run(&mut self) -> BDEResult<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let (run_tx, mut run_rx) = mpsc::unbounded_channel();
        let (search_data_tx, mut search_data_rx) = mpsc::unbounded_channel();
        let (data_tx, mut data_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut runp = true;
            let mut get_datap = true;

            while runp {
                if let Ok(data) = run_rx.try_recv() {
                    runp = data;
                };

                if let Ok(data) = search_data_rx.try_recv() {
                    get_datap = data;
                };

                if get_datap {
                    let test_path_1 = "~/";
                    // let test_path_2 = "~/AndroidStudioProjects/";
                    let search_path = Path::new(test_path_1);
                    match search_all_git_repo(search_path).await {
                        Ok(res) => {
                            data_tx.send(res).unwrap();
                        }
                        Err(_) => {
                            data_tx.send((Vec::new(), 0)).unwrap();
                        }
                    }
                    get_datap = false;
                }
            }
        });

        while self.runp {
            if let Ok(data) = data_rx.try_recv() {
                self.repos = data.0;
                self.component_repos_show.refresh_repop = false;
            }

            if let Some(action) = self.handle_events()? {
                match action {
                    AppAction::Quit => {
                        run_tx.send(false)?;
                        self.runp = false;
                        break;
                    }
                    AppAction::StartRefresh => {
                        if !self.component_repos_show.refresh_repop {
                            self.component_repos_show.refresh_repop = true;
                            search_data_tx.send(true)?;
                        }
                    }
                    AppAction::StartFilter => {
                        if !self.component_repos_show.refresh_repop {
                            self.run_mode = AppMode::Editing;
                        }
                    }
                    AppAction::ExitFilter => {
                        self.run_mode = AppMode::Normal;
                    }
                    _ => {}
                }
            }

            self.component_repos_show
                .update_show_repos(&self.repos, &self.component_input.input)?;

            terminal.draw(|f| match self.ui(f) {
                Ok(_) => {}
                Err(err) => {
                    panic!("{}", err);
                }
            })?;
        }

        Ok(())
    }
}

pub async fn run() -> BDEResult<()> {
    let mut app = App {
        repos: Vec::new(),
        runp: true,
        run_mode: AppMode::Normal,
        component_input: Input::new(),
        component_repos_show: ReposShow::new(),
    };

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    app.run().await?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

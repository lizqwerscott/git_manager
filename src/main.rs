use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, stdout};
use tokio::sync::mpsc;
use std::path::Path;

use git_manager::{BDEResult, GitRepo};
use git_manager::search_all_git_repo;

#[derive(Debug, Clone, Copy)]
enum AppAction {
    None,
    StartRefresh,
    StartFilter,
    SelectNext,
    SelectPervious,
    Quit,
}

#[derive(Debug)]
struct App {
    refresh_repop: bool,
    repos: Vec<GitRepo>,
    runp: bool,
}

impl App {
    fn handle_events(&mut self) -> BDEResult<AppAction> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(match key.code {
                        KeyCode::Char('q') => AppAction::Quit,
                        KeyCode::Char('g') => AppAction::StartRefresh,
                        KeyCode::Char('f') => AppAction::StartFilter,
                        KeyCode::Char('j') => AppAction::SelectNext,
                        KeyCode::Char('k') => AppAction::SelectPervious,
                        _ => AppAction::None,
                    });
                }
            }
        }

        Ok(AppAction::None)
    }

    fn ui(&self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(frame.size());

        // frame.render_widget(
        //     Block::new().borders(Borders::TOP).title("Search"),
        //     main_layout[0],
        // );

        frame.render_widget(
            Paragraph::new(format!(
                "Status: {}",
                self.refresh_repop
            )),
            main_layout[0],
        );

        let mut table_rows = Vec::new();

        for repo in &self.repos {
            let name = repo.name.clone();
            let status = repo.status.to_string();

            table_rows.push(Row::new(vec![name, status]));
        }

        frame.render_widget(
            Table::new(table_rows)
                .header(
                    Row::new(vec!["仓库名字", "仓库状态"])
                        .style(Style::default().fg(Color::Yellow))
                        // If you want some space between the header and the rest of the rows, you can always
                        // specify some margin at the bottom.
                        .bottom_margin(1),
                )
                .style(Style::default().fg(Color::White))
                .block(Block::default().title("仓库").borders(Borders::ALL))
                .widths(&[Constraint::Length(20), Constraint::Length(20)])
                // ...and they can be separated by a fixed spacing.
                .column_spacing(1)
                // If you wish to highlight a row in any specific way when it is selected...
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                // ...and potentially show a symbol in front of the selection.
                .highlight_symbol(">>"),
            main_layout[1],
        );
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
                match run_rx.try_recv() {
                    Ok(data) => {
                        runp = data;
                    }
                    Err(_) => {}
                };

                match search_data_rx.try_recv() {
                    Ok(data) => {
                        get_datap = data;
                    }
                    Err(_) => {}
                };

                if get_datap {
                    let test_path_2 = "~/AndroidStudioProjects/";
                    let search_path = Path::new(test_path_2);
                    match search_all_git_repo(search_path).await {
                        Ok(res) => {
                            data_tx.send(res);
                        },
                        Err(err) => {
                            data_tx.send((Vec::new(), 0));
                        }
                    }
                    get_datap = false;
                }
            }
        });

        while self.runp {
            match data_rx.try_recv() {
                Ok(data) => {
                    self.repos = data.0;
                    self.refresh_repop = false;
                }
                Err(_) => {}
            }

            terminal.draw(|f| self.ui(f))?;

            match self.handle_events()? {
                AppAction::Quit => {
                    run_tx.send(false)?;
                    self.runp = false
                }
                AppAction::StartRefresh => {
                    if !self.refresh_repop {
                        self.refresh_repop = true;
                        search_data_tx.send(true)?;
                    }
                }
                AppAction::StartFilter => {}
                _ => {}
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> BDEResult<()> {
    let mut app = App {
        refresh_repop: true,
        repos: Vec::new(),
        runp: true,
    };

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    app.run().await?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

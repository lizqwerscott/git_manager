use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, stdout};
use tokio::sync::mpsc;

use git_manager::BDEResult;

#[derive(Debug, Clone, Copy)]
enum AppAction {
    No,
    StartCounter,
    EndCounter,
    Quit,
}

#[derive(Debug)]
struct App {
    counter: u64,
    counterp: bool,
    runp: bool,
}

impl App {
    fn handle_events(&mut self) -> BDEResult<AppAction> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(match key.code {
                        KeyCode::Char('q') => AppAction::Quit,
                        KeyCode::Char('s') => AppAction::StartCounter,
                        KeyCode::Char('k') => AppAction::EndCounter,
                        _ => AppAction::No,
                    });
                }
            }
        }

        Ok(AppAction::No)
    }

    fn ui(&self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(frame.size());

        frame.render_widget(
            Block::new().borders(Borders::TOP).title("Search"),
            main_layout[0],
        );

        frame.render_widget(
            Paragraph::new(format!(
                "Press s or k to start or stop counter.\n\nCounter startp: {}\n\nCounter: {}",
                self.counterp, self.counter
            )),
            main_layout[1],
        );
    }

    async fn run(&mut self) -> BDEResult<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let (run_tx, mut run_rx) = mpsc::unbounded_channel();
        let (counter_tx, mut counter_rx) = mpsc::unbounded_channel();
        let (data_tx, mut data_rx) = mpsc::unbounded_channel();

        let temp_data = self.counter;

        tokio::spawn(async move {
            let mut temp_counter: u64 = temp_data;
            let mut runp = true;
            let mut counterp = false;

            while runp {
                match run_rx.try_recv() {
                    Ok(data) => {
                        runp = data;
                    }
                    Err(_) => {}
                };

                match counter_rx.try_recv() {
                    Ok(data) => {
                        counterp = data;
                    }
                    Err(_) => {}
                };

                if counterp {
                    temp_counter += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    data_tx.send(temp_counter);
                }
            }
        });

        while self.runp {
            match data_rx.try_recv() {
                Ok(data) => {
                    self.counter = data;
                }
                Err(_) => {}
            }

            terminal.draw(|f| self.ui(f))?;

            match self.handle_events()? {
                AppAction::Quit => {
                    run_tx.send(false)?;
                    self.runp = false
                }
                AppAction::StartCounter => {
                    self.counterp = true;
                    counter_tx.send(true)?;
                }
                AppAction::EndCounter => {
                    self.counterp = false;
                    counter_tx.send(false)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> BDEResult<()> {
    let mut app = App {
        counter: 0,
        counterp: false,
        runp: true,
    };

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    app.run().await?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

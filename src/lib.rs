use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::path::Path;
use tokio::sync::mpsc;

mod gitrepo;
pub mod utils;

use gitrepo::search_all_git_repo;
use gitrepo::GitRepo;
use utils::BDEResult;

#[derive(Debug, Clone, Copy)]
enum AppAction {
    None,
    StartRefresh,
    StartFilter,
    SelectNext,
    SelectPervious,
    Quit,
}

#[derive(Debug, Clone, Copy)]
enum AppMode {
    Normal,
    Editing,
}

#[derive(Debug)]
struct App {
    refresh_repop: bool,
    repos: Vec<GitRepo>,
    runp: bool,

    run_mode: AppMode,
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    cursor_position: usize,
}

impl App {
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn handle_events(&mut self) -> BDEResult<AppAction> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match self.run_mode {
                        AppMode::Normal => {
                            return Ok(match key.code {
                                KeyCode::Char('q') => AppAction::Quit,
                                KeyCode::Char('g') => AppAction::StartRefresh,
                                KeyCode::Char('f') => AppAction::StartFilter,
                                KeyCode::Char('j') => AppAction::SelectNext,
                                KeyCode::Char('k') => AppAction::SelectPervious,
                                _ => AppAction::None,
                            });
                        }
                        AppMode::Editing => {
                            match key.code {
                                KeyCode::Esc => self.run_mode = AppMode::Normal,
                                KeyCode::Char(to_insert) => {
                                    self.enter_char(to_insert);
                                }
                                KeyCode::Backspace => {
                                    self.delete_char();
                                }
                                KeyCode::Left => {
                                    self.move_cursor_left();
                                }
                                KeyCode::Right => {
                                    self.move_cursor_right();
                                }
                                _ => {}
                            };
                        }
                    }
                }
            }
        }

        Ok(AppAction::None)
    }

    fn ui(&self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(frame.size());

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
        frame.render_widget(Paragraph::new(text), main_layout[0]);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.run_mode {
                AppMode::Normal => Style::default(),
                AppMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("Input"));
        frame.render_widget(input, main_layout[1]);

        match self.run_mode {
            AppMode::Normal => {}
            AppMode::Editing => {
                frame.set_cursor(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    main_layout[1].x + self.cursor_position as u16 + 1,
                    // Move one line down, from the border to the input line
                    main_layout[1].y + 1,
                )
            }
        }

        if self.repos.is_empty() {
            let repo_message = if self.refresh_repop {
                "正在查找 Git 仓库..."
            } else {
                "需要刷新仓库"
            };

            frame.render_widget(
                Paragraph::new(repo_message)
                    .block(Block::default().title("仓库").borders(Borders::ALL)),
                main_layout[2],
            );
        } else {
            let mut table_rows = Vec::new();

            for repo in &self.repos {
                let name = repo.name.clone();
                let repo_path = repo.path.display().to_string();
                let mut path: Vec<&str> = repo_path.split('/').collect();
                if path.len() >= 2 {
                    path.drain(..3);
                }
                path.insert(0, "~");
                let status = repo.status.to_string();

                if name.to_lowercase().contains(&self.input) {
                    table_rows.push(Row::new(vec![name, path.join("/"), status]));
                }
            }

            frame.render_widget(
                Table::new(table_rows)
                    .header(
                        Row::new(vec!["仓库名字", "仓库路径", "仓库状态"])
                            .style(Style::default().fg(Color::Yellow))
                            // If you want some space between the header and the rest of the rows, you can always
                            // specify some margin at the bottom.
                            .bottom_margin(1),
                    )
                    .style(Style::default().fg(Color::White))
                    .block(Block::default().title("仓库").borders(Borders::ALL))
                    .widths(&[
                        Constraint::Length(20),
                        Constraint::Length(40),
                        Constraint::Length(20),
                    ])
                    // ...and they can be separated by a fixed spacing.
                    .column_spacing(1)
                    // If you wish to highlight a row in any specific way when it is selected...
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    // ...and potentially show a symbol in front of the selection.
                    .highlight_symbol(">>"),
                main_layout[2],
            );
        };
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
                    let test_path_1 = "~/";
                    let test_path_2 = "~/AndroidStudioProjects/";
                    let search_path = Path::new(test_path_1);
                    match search_all_git_repo(search_path).await {
                        Ok(res) => {
                            data_tx.send(res);
                        }
                        Err(_) => {
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
                AppAction::StartFilter => {
                    if !self.refresh_repop {
                        self.run_mode = AppMode::Editing;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

pub async fn run() -> BDEResult<()> {
    let mut app = App {
        refresh_repop: true,
        repos: Vec::new(),
        runp: true,
        run_mode: AppMode::Normal,
        cursor_position: 0,
        input: String::from(""),
    };

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    app.run().await?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

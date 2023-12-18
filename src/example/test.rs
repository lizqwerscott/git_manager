use std::path::Path;

use git_manager::{search_all_git_repo, BDEResult, GitRepo};

// #[tokio::main]
// async fn main() {
//     let test_path_1 = "~/";
//     let test_path_2 = "~/AndroidStudioProjects/";

//     let search_path = Path::new(test_path_1);

//     let mut repos = search_all_git_repo(search_path).await.unwrap();

//     println!("最终数量: {}: 报错数量: {}", repos.0.len(), repos.1);
//     println!("{:16}: {:10}", "仓库名字", "状态");
//     for repo in repos.0.iter_mut() {
//         println!("{}", repo);
//     }
// }

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, stdout};

#[tokio::main]
async fn main() -> io::Result<()> {
    let test_path_1 = "~/";
    let test_path_2 = "~/AndroidStudioProjects/";

    let search_path = Path::new(test_path_1);

    let mut repos = search_all_git_repo(search_path).await.unwrap();

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(&repos.0, f))?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(repos: &Vec<GitRepo>, frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(frame.size());

    frame.render_widget(
        Block::new().borders(Borders::TOP).title("Search"),
        main_layout[0],
    );

    let mut table_rows = Vec::new();

    for repo in repos {
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
            .widths(&[
                Constraint::Length(20),
                Constraint::Length(20),
            ])
            // ...and they can be separated by a fixed spacing.
            .column_spacing(1)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>"),
        main_layout[1],
    );

}

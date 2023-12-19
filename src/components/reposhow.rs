use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

#[derive(Debug)]
pub struct ReposShow {
    pub show_repos: Vec<(String, String, String)>,
    pub refresh_repop: bool,
}

impl ReposShow {
    pub fn new() -> Self {
        ReposShow {
            show_repos: Vec::new(),
            refresh_repop: true,
        }
    }
}

impl Component for ReposShow {
    fn handle_events(&mut self, event: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(match event.code {
            KeyCode::Char('g') => Some(AppAction::StartRefresh),
            KeyCode::Char('f') => Some(AppAction::StartFilter),
            KeyCode::Char('j') => Some(AppAction::SelectNext),
            KeyCode::Char('k') => Some(AppAction::SelectPervious),
            _ => None,
        })
    }

    fn draw(&mut self, _: AppMode, f: &mut Frame<'_>, rect: Rect) -> BDEResult<()> {
        if self.show_repos.is_empty() {
            let repo_message = if self.refresh_repop {
                "正在查找 Git 仓库..."
            } else {
                "需要刷新仓库"
            };

            f.render_widget(
                Paragraph::new(repo_message)
                    .block(Block::default().title("仓库").borders(Borders::ALL)),
                rect,
            );
        } else {
            let mut table_rows = Vec::new();

            for repo in &self.show_repos {
                table_rows.push(Row::new(vec![
                    repo.0.clone(),
                    repo.1.clone(),
                    repo.2.clone(),
                ]));
            }

            f.render_widget(
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
                rect,
            );
        };

        Ok(())
    }
}

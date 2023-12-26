use std::str::FromStr;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

use crate::gitrepo::{GitRepo, GitStatus};

#[derive(Debug)]
pub struct ReposShow {
    pub show_repos: Vec<(usize, String, String, String)>,
    pub refresh_repop: bool,
    pub state: TableState,
}

impl ReposShow {
    pub fn new() -> Self {
        ReposShow {
            show_repos: Vec::new(),
            refresh_repop: true,
            state: TableState::default(),
        }
    }

    pub fn update_show_repos(&mut self, repos: &[GitRepo], input: &str) -> BDEResult<()> {
        let mut use_path_search = false;
        let mut use_match_case = false;
        let mut filter_key: Vec<GitStatus> = Vec::new();
        let mut other_search: Vec<&str> = Vec::new();

        let key_lst: Vec<&str> = input.trim().split(' ').collect();

        for key in key_lst {
            if key == "+path" {
                use_path_search = true;
                continue;
            }

            if key == "+match_case" {
                use_match_case = true;
                continue;
            }

            if key.len() > 1 && key.starts_with('+') {
                if let Ok(filter_status) = GitStatus::from_str(&key[1..]) {
                    filter_key.push(filter_status);
                } else {
                    other_search.push(key);
                }
            } else {
                other_search.push(key);
            }
        }

        // let search_key = other_search.join(" ");

        self.show_repos.clear();
        for (index, repo) in repos.iter().enumerate() {
            let name = repo.name.clone();
            let repo_path = repo.path.display().to_string();
            let mut path: Vec<&str> = repo_path.split('/').collect();
            if path.len() >= 2 {
                path.drain(..3);
            }
            path.insert(0, "~");
            let status = repo.status.to_string();

            if !input.is_empty() {
                let filter_status_inp = if filter_key.is_empty() {
                    true
                } else {
                    filter_key.iter().any(|item| *item == repo.status)
                };

                let search_item = if use_path_search {
                    path.join("/")
                } else {
                    name.clone()
                };

                if !filter_status_inp {
                    continue;
                }

                let mut contain_allp = true;

                for search_key in &other_search {
                    if use_match_case {
                        if !search_item.contains(search_key) {
                            contain_allp = false;
                            break;
                        }
                    } else {
                        if !search_item
                            .to_lowercase()
                            .contains(&search_key.to_lowercase())
                        {
                            contain_allp = false;
                            break;
                        }
                    }
                }

                if !contain_allp {
                    continue;
                }
            }

            self.show_repos
                .push((index, name, path.join("/"), status.to_string()));
        }

        Ok(())
    }

    pub fn get_select_repo_id(&self) -> Option<usize> {
        let show_repo_index = self.state.selected()?;
        Some(self.show_repos[show_repo_index].0)
    }

    pub fn next(&mut self) {
        if self.show_repos.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.show_repos.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.show_repos.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.show_repos.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Component for ReposShow {
    fn handle_events(&mut self, event: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(match event.code {
            KeyCode::Char('g') => Some(AppAction::StartRefresh),
            KeyCode::Char('f') => Some(AppAction::StartFilter),
            KeyCode::Char('j') => Some(AppAction::SelectNext),
            KeyCode::Char('k') => Some(AppAction::SelectPervious),
            KeyCode::Char('y') => Some(AppAction::SelectCopyPath),
            KeyCode::Enter => Some(AppAction::SelectEnter),
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

            for (index, repo) in self.show_repos.iter().enumerate() {
                table_rows.push(Row::new(vec![
                    format!("{}", index),
                    repo.1.clone(),
                    repo.2.clone(),
                    repo.3.clone(),
                ]));
            }

            let selected_style = Style::default().add_modifier(Modifier::REVERSED);

            let header_cells = ["ID", "仓库名字", "仓库路径", "仓库状态"];
            let header = Row::new(header_cells)
                .style(Style::default().fg(Color::Yellow))
                .height(1)
                .bottom_margin(1);

            let t = Table::new(table_rows)
                .header(header)
                .style(Style::default().fg(Color::White))
                .block(Block::default().title("仓库").borders(Borders::ALL))
                .widths(&[
                    Constraint::Length(5),
                    Constraint::Length(20),
                    Constraint::Length(50),
                    Constraint::Length(20),
                ])
                // ...and they can be separated by a fixed spacing.
                .column_spacing(1)
                // If you wish to highlight a row in any specific way when it is selected...
                .highlight_style(selected_style)
                // ...and potentially show a symbol in front of the selection.
                .highlight_symbol(">>");

            f.render_stateful_widget(t, rect, &mut self.state);
        };

        Ok(())
    }
}

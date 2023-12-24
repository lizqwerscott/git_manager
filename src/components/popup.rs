use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

#[derive(Debug)]
pub struct CompletionPopup {
    pub show: bool,
    pub state: ListState,
    pub completions: Vec<(String, String)>,
}

impl CompletionPopup {
    pub fn default() -> Self {
        CompletionPopup {
            show: false,
            state: ListState::default(),
            completions: Vec::new(),
        }
    }

    pub fn get_select(&self) -> Option<String> {
        let i = self.state.selected()?;
        self.completions.get(i).map(|item| item.1.clone())
    }

    pub fn get_select_str(&self) -> Option<&str> {
        let i = self.state.selected()?;
        self.completions.get(i).map(|item| item.1.as_str())
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.completions.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.completions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Component for CompletionPopup {
    fn handle_events(&mut self, key: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(match key.code {
            KeyCode::Tab => {
                self.next();
                None
            }
            KeyCode::BackTab => {
                self.previous();
                None
            }
            KeyCode::Enter => {
                self.show = false;
                Some(AppAction::ComplectionFinish)
            }
            _ => None,
        })
    }

    fn draw(&mut self, _: AppMode, f: &mut Frame<'_>, rect: Rect) -> BDEResult<()> {
        let items: Vec<ListItem> = self
            .completions
            .iter()
            .map(|item| ListItem::new(Line::from(vec![item.0.as_str().into()])))
            .collect();

        let select_style = Style::new().add_modifier(Modifier::REVERSED);
        // let select_style = Style::new().fg(Color::Green);

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(select_style)
            .highlight_symbol("")
            .repeat_highlight_symbol(true);

        f.render_widget(Clear, rect); //this clears out the background
        f.render_stateful_widget(list, rect, &mut self.state);
        Ok(())
    }
}

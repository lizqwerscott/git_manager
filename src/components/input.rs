use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

#[derive(Debug)]
pub struct Input {
    pub input: String,
    /// Position of cursor in the editor area.
    cursor_position: usize,
}

impl Input {
    pub fn new() -> Self {
        Input {
            input: String::from(""),
            cursor_position: 0,
        }
    }

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

    // fn reset_cursor(&mut self) {
    //     self.cursor_position = 0;
    // }
}

impl Component for Input {
    fn handle_events(&mut self, key: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(match key.code {
            KeyCode::Esc => Some(AppAction::ExitFilter),
            KeyCode::Char(to_insert) => {
                self.enter_char(to_insert);
                None
            }
            KeyCode::Backspace => {
                self.delete_char();
                None
            }
            KeyCode::Left => {
                self.move_cursor_left();
                None
            }
            KeyCode::Right => {
                self.move_cursor_right();
                None
            }
            _ => None,
        })
    }
    fn draw(
        &mut self,
        mode: AppMode,
        f: &mut ratatui::prelude::Frame<'_>,
        rect: ratatui::prelude::Rect,
    ) -> BDEResult<()> {
        let input = Paragraph::new(self.input.as_str())
            .style(match mode {
                AppMode::Normal => Style::default(),
                AppMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, rect);

        match mode {
            AppMode::Normal => {}
            AppMode::Editing => {
                f.set_cursor(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    rect.x + self.cursor_position as u16 + 1,
                    // Move one line down, from the border to the input line
                    rect.y + 1,
                )
            }
        }
        Ok(())
    }
}
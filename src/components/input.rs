use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use super::popup::{CompletionItem, CompletionPopup};
use super::Component;
use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

#[derive(Debug)]
pub struct Input {
    pub input: String,
    /// Position of cursor in the editor area.
    cursor_position: usize,

    component_popup: CompletionPopup,
}

impl Input {
    pub fn new() -> Self {
        Input {
            input: String::from(""),
            cursor_position: 0,
            component_popup: CompletionPopup::default(),
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_left_n(&mut self, n: usize) {
        let cursor_moved_left = self.cursor_position.saturating_sub(n);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn move_cursor_right_n(&mut self, n: usize) {
        let cursor_moved_right = self.cursor_position.saturating_add(n);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_string(&mut self, new_str: &str) {
        self.input.push_str(new_str);
        self.move_cursor_right_n(new_str.len());
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_n_char(&mut self, n: usize) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost && self.cursor_position + 1 >= n {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - n;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left_n(n);
        }
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

    // fn clear_input(&mut self) {
    //     self.input = String::from("");
    //     self.reset_cursor();
    // }

    fn calc_item_score(input: &str, item: &str) -> u16 {
        if input.is_empty() {
            return 1;
        }

        let mut score = 0;
        let mut new_pos: usize = 0;
        let item_find_str = item.to_lowercase();

        for c in input.chars() {
            if let Some(pos) = item_find_str[new_pos..].find(c.to_ascii_lowercase()) {
                new_pos = pos + 1;
                let item_char = item.chars().nth(pos).unwrap();
                score += (item.len() as u16 - pos as u16 + 1) / item.len() as u16;
                if c == item_char {
                    score += 2;
                } else {
                    score += 1;
                }
            } else {
                return 0;
            }
        }

        score
    }

    pub fn update_complection(&mut self) -> BDEResult<()> {
        let complection_all = vec![
            String::from("path"),
            String::from("match_case"),
            String::from("NeedPull"),
            String::from("Clean"),
            String::from("NeedPush"),
            String::from("NeedCommit"),
            String::from("Timeout"),
        ];

        if self.input.is_empty() {
            self.component_popup.completions.clear();
            return Ok(());
        }

        if self.input.ends_with(' ') {
            self.component_popup.completions.clear();
            return Ok(());
        }

        if self.component_popup.complection_finish {
            return Ok(());
        }

        let input_split: Vec<&str> = self.input.split(' ').collect();

        if let Some(last_input) = input_split.last() {
            if let Some(stripped) = last_input.strip_prefix('+') {
                let filter_complection_input = stripped;

                let mut filter_complections: Vec<CompletionItem> = complection_all
                    .into_iter()
                    .filter_map(|item| {
                        let score = Input::calc_item_score(filter_complection_input, &item);
                        if score == 0 {
                            None
                        } else {
                            Some(CompletionItem {
                                score,
                                text: item.clone(),
                            })
                        }
                    })
                    .collect();

                filter_complections.sort_by_key(|item| item.score);
                filter_complections.reverse();

                if !filter_complections.is_empty() {
                    if self.component_popup.get_select().is_none() {
                        self.component_popup.state.select(Some(0));
                    }
                    self.component_popup.input_len = filter_complection_input.len();
                    self.component_popup.completions = filter_complections.clone();
                }
            }
        }

        Ok(())
    }
}

impl Component for Input {
    fn handle_events(&mut self, key: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(match key.code {
            KeyCode::Esc => Some(AppAction::ExitFilter),
            KeyCode::Char(to_insert) => {
                self.component_popup.complection_finish = false;
                self.enter_char(to_insert);
                None
            }
            KeyCode::Backspace => {
                self.component_popup.complection_finish = false;
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
            _ => {
                if self.component_popup.showp() {
                    if let Some(AppAction::ComplectionFinish) =
                        self.component_popup.handle_events(key)?
                    {
                        if let Some(input_text) = self.component_popup.get_select() {
                            self.delete_n_char(self.component_popup.input_len);
                            self.enter_string(&input_text);
                            self.component_popup.completions.clear();
                        }
                    }
                }

                None
            }
        })
    }
    fn draw(
        &mut self,
        mode: AppMode,
        f: &mut ratatui::prelude::Frame<'_>,
        rect: ratatui::prelude::Rect,
    ) -> BDEResult<()> {
        // let completion_preview_text = match self.component_popup.get_select_str() {
        //     Some(text) => {
        //         if self.component_popup.showp() {
        //             text
        //         } else {
        //             ""
        //         }
        //     }
        //     None => "",
        // };
        let completion_preview_text = "";

        let text = vec![
            self.input.as_str().into(),
            Span::styled(completion_preview_text, Style::new().bold()),
        ];

        let input = Paragraph::new(Line::from(text))
            .style(match mode {
                AppMode::Normal => Style::default(),
                // AppMode::Editing => Style::default().bg(Color::Yellow),
                AppMode::Editing => Style::default(),
            })
            .block(Block::default().borders(Borders::ALL).title("Filter"));
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

        // 需要在整个区域内最后绘制, 否则会被覆盖
        if self.component_popup.showp()
            && mode == AppMode::Editing
            && !self.component_popup.complection_finish
        {
            let mut need_height =
                (self.component_popup.completions.len() as f32 * 2.0).round() as u16;

            if need_height < 4 {
                need_height = 4;
            }

            if need_height > 10 {
                need_height = 10;
            }

            let area = Rect::new(
                rect.x + self.cursor_position as u16 + 1,
                rect.y + 2,
                20,
                need_height,
            );
            self.component_popup.draw(mode, f, area)?;
        }
        Ok(())
    }
}

use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::states::AppMode;
use crate::utils::BDEResult;

#[derive(Debug)]
pub struct StatusBar {
    pub search_repo_duration: f64,
    pub show_repo_len: usize,
    pub all_repo_len: usize,
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            search_repo_duration: 0.0,
            show_repo_len: 0,
            all_repo_len: 0,
        }
    }
}

impl Component for StatusBar {
    fn draw(&mut self, mode: AppMode, f: &mut Frame<'_>, rect: Rect) -> BDEResult<()> {
        let status_bar_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rect);

        let (msg, style) = match mode {
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
        f.render_widget(Paragraph::new(text), status_bar_layout[0]);

        let use_time = format!("search time: {}s", self.search_repo_duration);
        let repo_number = if self.all_repo_len == 0 {
            String::from("repo: 0")
        } else {
            format!("repo: {}/{}", self.show_repo_len, self.all_repo_len)
        };

        let text = Text::from(Line::from(vec![
            use_time.into(),
            " | ".into(),
            repo_number.into(),
        ]));
        f.render_widget(Paragraph::new(text), status_bar_layout[1]);

        Ok(())
    }
}

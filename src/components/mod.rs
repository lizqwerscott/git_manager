use crossterm::event::KeyEvent;
use ratatui::prelude::{Frame, Rect};

pub mod input;
pub mod reposhow;
pub mod statusbar;

use crate::states::{AppAction, AppMode};
use crate::utils::BDEResult;

pub trait Component {
    #[allow(unused_variables)]
    fn handle_events(&mut self, event: KeyEvent) -> BDEResult<Option<AppAction>> {
        Ok(None)
    }
    #[allow(unused_variables)]
    fn update(&mut self, mode: AppMode, action: AppAction) -> BDEResult<Option<AppAction>> {
        Ok(None)
    }
    fn draw(&mut self, mode: AppMode, f: &mut Frame<'_>, rect: Rect) -> BDEResult<()>;
}

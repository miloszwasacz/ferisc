use super::NavInfoBuilder;
use crate::core::Stats;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;

pub struct Dashboard(Stats);

impl Dashboard {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> EventResult {
        match event {
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => EventResult::Tick,
            event => EventResult::Ok,
        }
    }
}

impl StatefulWidget for &Dashboard {
    type State = NavInfoBuilder;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.push_back(("Enter", "Tick"));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventResult {
    Ok,
    Tick,
}

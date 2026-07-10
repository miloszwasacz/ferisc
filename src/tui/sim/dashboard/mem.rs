use super::Focused;
use crate::core::mem::Stats;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct MemSubsystem(Stats);

impl MemSubsystem {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }
}

impl<'a> StatefulWidget for &'a MemSubsystem {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

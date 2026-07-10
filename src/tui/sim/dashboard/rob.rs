use super::Focused;
use crate::core::ee::rob::Stats;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct Rob(Stats);

impl Rob {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }
}

impl<'a> StatefulWidget for &'a Rob {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

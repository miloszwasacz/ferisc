use super::Focused;
use crate::core::fe::predecode::Stats;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct Predecode(Stats);

impl Predecode {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }
}

impl<'a> StatefulWidget for &'a Predecode {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

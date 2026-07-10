use super::Focused;
use crate::core::fe::decode::Stats;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct Decode(Stats);

impl Decode {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }
}

impl<'a> StatefulWidget for &'a Decode {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

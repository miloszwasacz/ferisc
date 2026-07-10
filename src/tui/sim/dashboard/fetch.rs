use super::Focused;
use crate::core::fe::{addr_gen, fetch};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct Fetch(addr_gen::Stats, fetch::Stats);

impl Fetch {
    pub fn new(addr_gen: addr_gen::Stats, fetch: fetch::Stats) -> Self {
        Self(addr_gen, fetch)
    }
}

impl<'a> StatefulWidget for &'a Fetch {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

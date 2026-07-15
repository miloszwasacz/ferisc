use super::{Focused, UopList};
use crate::core::fe::decode::Stats;
use crate::tui::sim::AddrView;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::{line, span};
use ratatui::widgets::StatefulWidget;

pub struct Decode(UopList<Stats>);

impl Decode {
    pub fn new(stats: Stats) -> Self {
        Self(UopList::new(stats))
    }

    pub fn update(&mut self, stats: Stats) {
        self.0.update(stats);
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> bool {
        self.0.handle_event(event)
    }
}

impl<'a> StatefulWidget for &'a Decode {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.0.render(
            |stats| {
                stats
                    .iter()
                    .map(|uop| line![span!("{:#}: ", AddrView(uop.addr)), uop.kind.as_span()])
            },
            area,
            buf,
            state,
        )
    }
}

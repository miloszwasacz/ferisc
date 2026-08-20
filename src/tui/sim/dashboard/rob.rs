use super::{Focused, UopList};
use crate::core::ee::rob::{Stats, entry::State};
use crate::mem::AddrView;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::{line, span};
use ratatui::style::Stylize;
use ratatui::widgets::StatefulWidget;

pub struct Rob(UopList<Stats>);

impl Rob {
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

impl<'a> StatefulWidget for &'a Rob {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.0.render(
            |stats| {
                stats.iter().map(|entry| {
                    let line = line![
                        span!("{:#}: ", AddrView(entry.addr)),
                        entry.display.as_span()
                    ];
                    match entry.state {
                        State::NotDone | State::Done => line,
                        State::Flushed => line.crossed_out().dark_gray(),
                    }
                })
            },
            area,
            buf,
            state,
        )
    }
}

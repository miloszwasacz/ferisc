use super::{Focused, UopList};
use crate::core::{fe::predecode::Stats, uop::predecode::PdcdBytes};
use crate::tui::sim::AddrView;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::{line, span};
use ratatui::style::Stylize;
use ratatui::widgets::StatefulWidget;

pub struct Predecode(UopList<Stats>);

impl Predecode {
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

impl<'a> StatefulWidget for &'a Predecode {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.0.render(
            |stats| {
                stats.iter().map(|pd| {
                    line![
                        span!("{}:", AddrView(pd.meta.addr)),
                        match &pd.bytes {
                            Ok(bytes) => match bytes {
                                PdcdBytes::Normal(bytes) => span!("{:032b}", bytes),
                                PdcdBytes::Compressed(bytes) => span!("{:016b}", bytes),
                            },
                            Err(e) => span!("{}", e).yellow(),
                        }
                    ]
                })
            },
            area,
            buf,
            state,
        )
    }
}

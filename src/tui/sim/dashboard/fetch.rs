use super::Focused;
use crate::core::fe::{addr_gen, fetch};
use crate::mem::AddrView;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::{line, span};
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, StatefulWidget, Widget};

pub struct Fetch {
    addr_gen: addr_gen::Stats,
    fetch: fetch::Stats,
}

impl Fetch {
    pub fn new(addr_gen: addr_gen::Stats, fetch: fetch::Stats) -> Self {
        Self { addr_gen, fetch }
    }

    pub fn update(&mut self, addr_gen: addr_gen::Stats, fetch: fetch::Stats) {
        self.addr_gen = addr_gen;
        self.fetch = fetch;
    }

    pub fn handle_event(&mut self, _: KeyEvent) -> bool {
        false
    }
}

impl<'a> StatefulWidget for &'a Fetch {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, _: &mut Self::State) {
        let width = AddrView::PRETTY_WIDTH;
        let text = Paragraph::new(vec![
            line![span!(
                "Next PC:    {:width$}",
                AddrView(self.addr_gen.next_pc)
            )],
            line![
                "Fetched PC: ",
                match self.fetch {
                    fetch::Stats::Idle => span!(""),
                    fetch::Stats::Fetching(pc)
                    | fetch::Stats::Fetched {
                        addr: pc,
                        exception: None,
                    } => span!("{:width$}", AddrView(pc)),
                    fetch::Stats::Fetched {
                        addr: pc,
                        exception: Some(_),
                    } => span!("{:width$}", AddrView(pc)).yellow(),
                }
            ],
        ]);
        Widget::render(text, area, buf);
    }
}

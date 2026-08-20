use super::Focused;
use crate::core::{
    mem::{Config, ReqStats, Src, Stats},
    uop::commit::Dest,
};
use crate::mem::AddrView;
use crate::tui::focus::{FOCUS_COLOR, Focusable};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Offset, Rect, Size, Spacing};
use ratatui::macros::{horizontal, line, span};
use ratatui::style::{Style, Stylize};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{
    Block, BorderType, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};
use std::cell::Cell;

const BORDERS_LEN: u16 = 2;

pub struct MemSubsystem {
    stats: Stats,
    min_height: u16,
    top: Cell<usize>,
}

impl MemSubsystem {
    pub fn new(stats: Stats, config: &Config) -> Self {
        let min_height = config.ldq_size.max(config.stq_size).get();
        Self {
            stats,
            min_height: min_height as _,
            top: Cell::new(0),
        }
    }

    pub fn update(&mut self, stats: Stats) {
        self.stats = stats;
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> bool {
        match event {
            key_event!(KeyCode::Up) => self.top.update(|i| i.saturating_sub(1)),
            key_event!(KeyCode::Down) => self.top.update(|i| {
                i.saturating_add(1)
                    .min(self.min_height.saturating_sub(1) as _)
            }),
            _ => return false,
        }
        true
    }
}

impl<'a> StatefulWidget for &'a MemSubsystem {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let req_area = area.resize(Size::new(area.width, 1));
        let queues_area = area.outer(Margin::new(1, 0)) + Offset::new(0, 1);

        // Queues
        let count = self.stats.loads().len().max(self.stats.stores().len());
        let max_top = count.checked_sub(queues_area.height.saturating_sub(BORDERS_LEN) as _);
        let scrolls = max_top.is_some();
        self.top.update(|x| x.min(max_top.unwrap_or_default()));

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .merge_borders(MergeStrategy::Fuzzy)
            .focus_if(state.is_some());
        let mut render_blocks = |ldq, stq| {
            block.clone().title_top("LDQ").render(ldq, buf);
            block.clone().title_top("STQ").render(stq, buf);
        };

        let (ldq_block_area, stq_block_area) = if !scrolls {
            let [ldq_block_area, stq_block_area] = horizontal![*=1, *=1]
                .spacing(Spacing::Overlap(1))
                .areas(queues_area);
            render_blocks(ldq_block_area, stq_block_area);

            (ldq_block_area, stq_block_area)
        } else {
            if let Some(state) = state.as_mut() {
                state.push_back(("↑↓", "Scroll"));
            }
            let mut scroll_state = ScrollbarState::default()
                .content_length(count)
                .position(self.top.get());

            let [ldq_block_area, middle_area, right_area] = horizontal![*=1, *=1, ==2]
                .spacing(Spacing::Overlap(1))
                .areas(queues_area);
            let stq_block_area = middle_area.union(right_area);
            let scrollbar_area = Rect {
                x: right_area.x,
                y: right_area.y.saturating_add(BORDERS_LEN / 2),
                width: 1,
                height: right_area.height.saturating_sub(BORDERS_LEN),
            };
            render_blocks(ldq_block_area, stq_block_area);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(if state.is_some() {
                    Style::default().fg(FOCUS_COLOR)
                } else {
                    Style::default().dark_gray()
                })
                .render(scrollbar_area, buf, &mut scroll_state);
            self.top.set(scroll_state.get_position());

            (ldq_block_area, middle_area)
        };
        let ldq_area = block.inner(ldq_block_area);
        let stq_area = block.inner(stq_block_area);

        let top = self.top.get();
        let get_addr = |addr| match addr {
            Some(addr) => span!("{}", AddrView(addr)),
            None => span!("?").red(),
        };
        macro_rules! ld_line {
            ($prefix:expr, $ld:expr, $addr:expr) => {{
                let signed = if $ld.signed { "" } else { "u" };
                let dest = match $ld.dest {
                    Dest::Int(dest) => span!("{}", dest.arch()),
                };
                line![
                    $prefix,
                    span!("l{}{:1} ", $ld.size, signed),
                    dest,
                    ", ",
                    $addr
                ]
            }};
        }
        macro_rules! st_line {
            ($prefix:expr, $st:expr, $addr:expr) => {{
                let src = match $st.src {
                    Src::Int(dest) => span!("{}", dest.arch()),
                };
                line![$prefix, span!("s{} ", $st.size), src, ", ", $addr]
            }};
        }

        let ldq = Paragraph::new(
            self.stats
                .loads()
                .iter()
                .enumerate()
                .skip(top)
                .map(|(i, ld)| ld_line!(span!("{:2>}: ", i + 1), ld, get_addr(ld.addr)))
                .collect::<Vec<_>>(),
        );
        Clear.render(ldq_area, buf);
        Widget::render(ldq, ldq_area, buf);

        let stq = Paragraph::new(
            self.stats
                .stores()
                .iter()
                .enumerate()
                .skip(top)
                .map(|(i, st)| st_line!(span!("{:2>}: ", i + 1), st, get_addr(st.addr)))
                .collect::<Vec<_>>(),
        );
        Clear.render(stq_area, buf);
        Widget::render(stq, stq_area, buf);

        // Current request
        let prefix = span!("Current: ");
        match &self.stats.req {
            ReqStats::None => line![prefix, span!("‒")],
            ReqStats::Ld(ld) => ld_line!(prefix, ld, span!("{}", AddrView(ld.addr))),
            ReqStats::St(st) => st_line!(prefix, st, span!("{}", AddrView(st.addr))),
        }
        .render(req_area, buf);
    }
}

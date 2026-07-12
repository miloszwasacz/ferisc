use super::Focused;
use crate::core::reg::{
    ArchRegCount,
    int::{IntReg, IntVal},
    phys::rat,
};
use crate::tui::components::scroll_view::{ScrollView, ScrollViewState};
use crossterm::event::{KeyCode, KeyEvent};
use generic_array::GenericArray;
use ratatui::buffer::Buffer;
use ratatui::layout::{Flex, Position, Rect, Size};
use ratatui::macros::horizontal;
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Row, StatefulWidget, Table, Widget};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

type Stats = rat::Stats<IntReg>;

pub struct IntRegs(Stats, RefCell<ScrollViewState>);

impl IntRegs {
    pub fn new(stats: Stats) -> Self {
        Self(stats, Default::default())
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> bool {
        let mut scroll = self.1.borrow_mut();
        let scroll_fn = match event {
            key_event!(KeyCode::Left) => ScrollViewState::scroll_left,
            key_event!(KeyCode::Right) => ScrollViewState::scroll_right,
            key_event!(KeyCode::Home) => {
                |scroll: &mut ScrollViewState| scroll.set_offset(Position::default())
            }
            key_event!(KeyCode::End) => {
                |scroll: &mut ScrollViewState| scroll.set_offset(Position::new(u16::MAX, 0))
            }
            _ => return false,
        };
        for _ in 0..(WIDTH + SPACING as usize) {
            scroll_fn(&mut scroll);
        }
        true
    }
}

const WIDTH: usize = 6;
const SPACING: u16 = 1;

impl<'a> StatefulWidget for &'a IntRegs {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(state) = state.as_mut() {
            state.extend([
                ("←→", "Scroll"),
                ("Home", "Scroll to start"),
                ("End", "Scroll to end"),
            ]);
        }
        let [header_area, table_area] = horizontal![==6, *=1].areas(area);

        let regs: &GenericArray<_, <IntReg as ArchRegCount>::COUNT> = self.0.deref();
        let widths = std::iter::repeat_n(WIDTH as u16, regs.len());
        let (names, spec, comm) = {
            let mut names = GenericArray::<_, <IntReg as ArchRegCount>::COUNT>::uninit();
            let mut spec = GenericArray::<_, <IntReg as ArchRegCount>::COUNT>::uninit();
            let mut comm = GenericArray::<_, <IntReg as ArchRegCount>::COUNT>::uninit();
            for (i, (r, s, c)) in regs.iter().enumerate() {
                let int_val = |val: &IntVal| {
                    let mut fmt = format!("{:>width$}", val.u(), width = WIDTH);
                    if fmt.len() > WIDTH {
                        fmt.replace_range(..fmt.len() - WIDTH, "…");
                    }
                    fmt
                };

                names[i].write(format!("{:>width$}", r, width = WIDTH));
                spec[i].write(
                    s.as_ref()
                        .map(int_val)
                        .unwrap_or_else(|| format!("{:>width$}", '?', width = WIDTH)),
                );
                comm[i].write(int_val(c));
            }

            unsafe {
                (
                    GenericArray::assume_init(names),
                    GenericArray::assume_init(spec),
                    GenericArray::assume_init(comm),
                )
            }
        };

        let mut scroll_view = ScrollView::new(Size::new(
            regs.len()
                .checked_sub(1)
                .map(|c| (WIDTH + SPACING as usize) * c + WIDTH)
                .unwrap_or(0) as _,
            table_area.height.saturating_sub(1),
        ))
        .horizontal_scrollbar_style(if state.as_ref().is_some() {
            Style::default().cyan()
        } else {
            Style::default()
        });
        let table = Table::new([Row::new(names), Row::new(spec), Row::new(comm)], widths)
            .column_spacing(SPACING)
            .flex(Flex::Start);

        Paragraph::new("\nSpec:\nComm:").render(header_area, buf);
        scroll_view.render_widget(table, scroll_view.area());
        scroll_view.render(table_area, buf, self.1.borrow_mut().deref_mut());
    }
}

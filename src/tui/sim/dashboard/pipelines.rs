use super::Focused;
use crate::core::{
    ee::pipeline::{AllStats, Configs, LineStats},
    reg::{
        int::IntReg,
        phys::{RegOper, Stats as PrfStats},
    },
};
use crate::tui::{
    components::scroll_view::{ScrollView, ScrollViewState},
    sim::AddrView,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Rect, Size, Spacing};
use ratatui::macros::{line, span, vertical};
use ratatui::style::{Style, Stylize};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, StatefulWidget, Widget};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::DerefMut;

const BORDERS_LEN: u16 = 2;
const LINE_WIDTH: u16 = BORDERS_LEN + 4 + 25;

pub struct Pipelines {
    stats: AllStats,
    line_count: usize,
    min_height: u16,
    scroll_view_state: RefCell<ScrollViewState>,
    selected: usize,
}

impl Pipelines {
    pub fn new(stats: AllStats, configs: &Configs) -> Self {
        let Configs { int } = configs;

        let line_count = int.lines.len();
        let min_height = int
            .lines
            .iter()
            .map(|(rs, _eu)| {
                //TODO EU height
                BORDERS_LEN.saturating_add(rs.size.get() as _)
            })
            .max()
            .unwrap_or(0);

        Self {
            stats,
            line_count,
            min_height,
            scroll_view_state: Default::default(),
            selected: 0,
        }
    }

    pub fn update(&mut self, stats: AllStats) {
        self.stats = stats;
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> bool {
        let mut scroll = self.scroll_view_state.borrow_mut();
        match event {
            key_event!(KeyCode::Up) => scroll.scroll_up(),
            key_event!(KeyCode::Down) => scroll.scroll_down(),
            key_event!(KeyCode::Left) => match self.selected.checked_sub(1) {
                Some(new) => {
                    self.selected = new;
                    for _ in 0..LINE_WIDTH {
                        scroll.scroll_left()
                    }
                }
                None => {
                    self.selected = self.line_count.saturating_sub(1);
                    for _ in 0..LINE_WIDTH as usize * self.line_count {
                        scroll.scroll_right();
                    }
                }
            },
            key_event!(KeyCode::Right) => {
                match self
                    .selected
                    .checked_add(1)
                    .and_then(|i| (i < self.line_count).then_some(i))
                {
                    Some(new) => {
                        self.selected = new;
                        for _ in 0..LINE_WIDTH {
                            scroll.scroll_right()
                        }
                    }
                    None => {
                        self.selected = 0;
                        for _ in 0..LINE_WIDTH as usize * self.line_count {
                            scroll.scroll_left();
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
}

impl<'a> StatefulWidget for &'a Pipelines {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let AllStats { int } = &self.stats;
        macro_rules! lines {
            ($pipeline:expr) => {{
                let p = $pipeline;
                p.lines().iter().map(|l| Line::from((l, &p.prf)))
            }};
        }

        let width = self.line_count as u16 * LINE_WIDTH;
        let height = area.height.max(self.min_height);
        let (scroll_view_width, line_width, height) = match area.width.cmp(&width) {
            Ordering::Less => (width, LINE_WIDTH, height.saturating_sub(1)),
            Ordering::Equal => (area.width, LINE_WIDTH, height),
            Ordering::Greater => (
                area.width,
                area.width.checked_div(self.line_count as _).unwrap_or(0),
                height,
            ),
        };
        if let Some(state) = state.as_mut() {
            let h = area.width < width;
            let v = area.height < self.min_height;
            state.push_back((
                match (h, v) {
                    (false, false) => return,
                    (false, true) => "↑↓",
                    (true, false) => "←→",
                    (true, true) => "↑↓←→",
                },
                "Scroll",
            ));
        }

        let mut scroll_view = ScrollView::new(Size::new(scroll_view_width, height))
            .scrollbars_style(if state.is_some() {
                Style::default().cyan()
            } else {
                Style::default()
            });
        let item_area = scroll_view.area().resize(Size::new(line_width, height));

        lines!(int)
            .enumerate()
            .map(|(i, l)| {
                let area = item_area + Offset::new(i as i32 * item_area.width as i32, 0);
                let l = if state.as_ref().is_some_and(|_| i == self.selected) {
                    l.select()
                } else {
                    l
                };
                (area, l)
            })
            .for_each(|(area, l)| scroll_view.render_widget(l, area));

        scroll_view.render(area, buf, self.scroll_view_state.borrow_mut().deref_mut())
    }
}

pub struct Line {
    rs: Paragraph<'static>,
    eu: Paragraph<'static>,
    selected: bool,
}

impl From<(&LineStats<IntReg>, &PrfStats<IntReg>)> for Line {
    fn from(((rs, eu), prf): (&LineStats<IntReg>, &PrfStats<IntReg>)) -> Self {
        use crate::core::uop::int::{EeUop as Uop, Src};
        let read_reg = |reg: &RegOper<_>| match prf[*reg].as_ref() {
            Some(val) => span!("{}", val.s()),
            None => span!("{}", reg.arch()).red(),
        };
        let read_src = |src: &Src<_>| match src {
            Src::Reg(reg) => read_reg(reg),
            Src::Imm(imm) => span!("{}", imm.s()),
        };

        let rs = Paragraph::new(
            rs.iter()
                .enumerate()
                .map(|(i, uop)| match &uop.kind {
                    Uop::Alu {
                        op,
                        src1,
                        src2,
                        dest,
                    } => {
                        let src1 = read_src(src1);
                        let src2 = read_src(src2);
                        line![
                            span!("{:>2}: {} {}, ", i + 1, op, dest.arch()),
                            src1,
                            ", ",
                            src2
                        ]
                    }
                    Uop::Mul {
                        op,
                        src1,
                        src2,
                        dest,
                    } => {
                        let src1 = read_reg(src1);
                        let src2 = read_reg(src2);
                        line![
                            span!("{:>2}: {} {}, ", i + 1, op, dest.arch()),
                            src1,
                            ", ",
                            src2
                        ]
                    }
                    Uop::Jalr { base, offset, .. } => {
                        let base = read_reg(base);
                        line![span!("{:>2}: jalr {}(", i + 1, offset.s()), base, ")"]
                    }
                    Uop::Br {
                        cmp,
                        src1,
                        src2,
                        target,
                        ..
                    } => {
                        let src1 = read_reg(src1);
                        let src2 = read_reg(src2);
                        line![
                            span!("{:>2}: b{} ", i + 1, cmp),
                            src1,
                            ", ",
                            src2,
                            span!(", {}", AddrView(*target))
                        ]
                    }
                    Uop::Lda { base, offset, .. } => {
                        let base = read_reg(base);
                        line![span!("{:>2}: lda {}(", i + 1, offset.s()), base, ")"]
                    }
                    Uop::Sta { base, offset, .. } => {
                        let base = read_reg(base);
                        line![span!("{:>2}: sta {}(", i + 1, offset.s()), base, ")"]
                    }
                    Uop::Csr { op, csr, src, dest } => {
                        let src = read_src(src);
                        line![
                            span!("{:>2}: csr{} {}, {}, ", i + 1, op, csr, dest.arch()),
                            src
                        ]
                    }
                })
                .collect::<Vec<_>>(),
        );
        //TODO EU
        let _ = eu;
        let eu = Paragraph::new("");

        Self {
            rs,
            eu,
            selected: false,
        }
    }
}

impl Line {
    pub fn select(mut self) -> Self {
        self.selected = true;
        self
    }
}

impl Widget for Line {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .merge_borders(MergeStrategy::Fuzzy);
        if self.selected {
            block = block.cyan();
        }

        //TODO Proper EU area
        let [rs_area, eu_area] = vertical![*=1, ==0].spacing(Spacing::Overlap(1)).areas(area);
        let rs_area_inner = block.inner(rs_area);
        let eu_area_inner = block.inner(eu_area);

        block.clone().render(rs_area, buf);
        block.render(eu_area, buf);
        Clear.render(rs_area_inner, buf);
        Clear.render(eu_area_inner, buf);

        self.rs.render(rs_area_inner, buf);
        self.eu.render(eu_area_inner, buf);
    }
}

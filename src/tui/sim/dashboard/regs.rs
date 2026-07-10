use super::Focused;
use crate::core::reg::{int::IntReg, phys::rat};
use ratatui::prelude::*;

type Stats = rat::Stats<IntReg>;

pub struct IntRegs(Stats);
pub struct Speculative<'a>(&'a IntRegs);
pub struct Committed<'a>(&'a IntRegs);

impl IntRegs {
    pub fn new(stats: Stats) -> Self {
        Self(stats)
    }

    pub fn speculative(&self) -> Speculative<'_> {
        Speculative(self)
    }

    pub fn committed(&self) -> Committed<'_> {
        Committed(self)
    }
}

impl<'a> StatefulWidget for Speculative<'a> {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

impl<'a> StatefulWidget for Committed<'a> {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

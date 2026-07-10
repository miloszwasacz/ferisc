use super::Focused;
use crate::core::ee::pipeline::AllStats;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

pub struct Pipelines(AllStats);

impl Pipelines {
    pub fn new(stats: AllStats) -> Self {
        Self(stats)
    }
}

impl<'a> StatefulWidget for &'a Pipelines {
    type State = Focused<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //TODO
    }
}

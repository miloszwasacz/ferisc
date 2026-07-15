use super::Focused;
use crate::tui::components::scroll_view::{ScrollView, ScrollViewState};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, StatefulWidget};
use std::cell::RefCell;
use std::ops::DerefMut;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UopList<S>(S, RefCell<ScrollViewState>);

impl<S> UopList<S> {
    pub fn new(stats: S) -> Self {
        Self(stats, Default::default())
    }

    pub fn update(&mut self, stats: S) {
        self.0 = stats;
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> bool {
        let mut scroll = self.1.borrow_mut();
        match event {
            key_event!(KeyCode::Up) => scroll.scroll_up(),
            key_event!(KeyCode::Down) => scroll.scroll_down(),
            key_event!(KeyCode::Left) => scroll.scroll_left(),
            key_event!(KeyCode::Right) => scroll.scroll_right(),
            _ => return false,
        }
        true
    }

    pub fn render<'a, F, I>(
        &'a self,
        into_lines: F,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Focused<'_>,
    ) where
        F: FnOnce(&'a S) -> I,
        I: IntoIterator<Item = Line<'a>>,
    {
        let lines = into_lines(&self.0).into_iter().collect::<Lines>();
        let mut nav_info = || {
            if let Some(state) = state.as_mut() {
                let h = lines.scrolls_horizontally(area);
                let v = lines.scrolls_vertically(area);
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
        };
        nav_info();

        lines.render(area, buf, self.1.borrow_mut().deref_mut());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Lines<'a>(Vec<Line<'a>>, Size);

impl<'a> Lines<'a> {
    pub fn new<I: IntoIterator<Item = Line<'a>>>(items: I) -> Self {
        let lines = items.into_iter().collect::<Vec<_>>();
        let size = Size::new(
            lines.iter().map(|l| l.width() as u16).max().unwrap_or(0),
            lines.len() as _,
        );
        Self(lines, size)
    }

    fn scrolls_horizontally(&self, area: Rect) -> bool {
        self.1.width > area.width
    }

    fn scrolls_vertically(&self, area: Rect) -> bool {
        self.1.height > area.height
    }
}

impl<'a> FromIterator<Line<'a>> for Lines<'a> {
    fn from_iter<T: IntoIterator<Item = Line<'a>>>(iter: T) -> Self {
        Self::new(iter)
    }
}

impl<'a> StatefulWidget for Lines<'a> {
    type State = ScrollViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines = Paragraph::new(self.0);
        let mut scroll_view = ScrollView::new(self.1);
        scroll_view.render_widget(lines, scroll_view.area());
        scroll_view.render(area, buf, state);
    }
}

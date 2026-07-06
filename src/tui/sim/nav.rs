use ratatui::layout::Flex;
use ratatui::macros::line;
use ratatui::prelude::*;
use std::borrow::Cow;
use std::collections::VecDeque;

//#region Item

#[derive(Debug, Clone)]
pub struct NavItem {
    action: Cow<'static, str>,
    desc: Cow<'static, str>,
}

impl NavItem {
    pub fn new<A, D>(action: A, desc: D) -> Self
    where
        A: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
    {
        Self {
            action: action.into(),
            desc: desc.into(),
        }
    }
}

impl Widget for NavItem {
    fn render(self, area: Rect, buf: &mut Buffer) {
        line![
            Span::raw(self.action).cyan(),
            Span::raw(" "),
            Span::raw(self.desc)
        ]
        .render(area, buf);
    }
}

impl<A, D> From<(A, D)> for NavItem
where
    A: Into<Cow<'static, str>>,
    D: Into<Cow<'static, str>>,
{
    fn from((action, desc): (A, D)) -> Self {
        Self::new(action, desc)
    }
}

//#endregion

//#region Info

#[derive(Debug, Clone)]
pub struct NavInfo(VecDeque<NavItem>);

impl Widget for NavInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut width = 0;
        let layout = Layout::horizontal(
            self.0
                .iter()
                .map(|l| l.action.len() + 1 + l.desc.len())
                .inspect(|w| width += w)
                .map(|w| Constraint::Length(w as _)),
        )
        .flex(Flex::Start)
        .spacing(
            (2..=5)
                .rev()
                .find(|w| (width + self.0.len() * w) <= area.width as usize)
                .unwrap_or(1) as i32,
        )
        .split(area);

        for (item, area) in self.0.into_iter().zip(layout.as_ref()) {
            item.render(*area, buf);
        }
    }
}

//#endregion

//#region Builder

#[derive(Debug, Clone, Default)]
pub struct NavInfoBuilder(VecDeque<NavItem>);

impl NavInfoBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push_back(&mut self, item: impl Into<NavItem>) {
        self.0.push_back(item.into());
    }

    pub fn push_front(&mut self, item: impl Into<NavItem>) {
        self.0.push_front(item.into());
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = impl Into<NavItem>>) {
        self.0.extend(items.into_iter().map(Into::into));
    }

    pub fn build(self) -> NavInfo {
        NavInfo(self.0)
    }
}

//#endregion

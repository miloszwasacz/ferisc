use const_format::concatcp;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
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

    pub fn into_spans(self) -> impl IntoIterator<Item = Span<'static>> {
        [
            Span::raw(self.action).cyan(),
            Span::raw(" "),
            Span::raw(self.desc),
        ]
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
        self.0
            .into_iter()
            .flat_map(|item| {
                let spacer = Span::raw(concatcp!(" ", symbols::DOT, " ")).dark_gray();
                std::iter::once(spacer).chain(item.into_spans())
            })
            .skip(1)
            .collect::<Line>()
            .render(area, buf);
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

use ratatui::style::{Color, Styled, Stylize};

pub const FOCUS_COLOR: Color = Color::LightCyan;

pub trait Focusable {
    fn focus_if(self, focus: bool) -> Self;
}
impl<T: Styled<Item = T>> Focusable for T {
    fn focus_if(self, focus: bool) -> Self {
        if focus { self.fg(FOCUS_COLOR) } else { self }
    }
}

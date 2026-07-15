use self::decode::Decode;
use self::fetch::Fetch;
use self::mem::MemSubsystem;
use self::pipelines::Pipelines;
use self::predecode::Predecode;
use self::regs::IntRegs;
use self::rob::Rob;
use self::uop_list::UopList;
use super::NavInfoBuilder;
use crate::core::{Config, Stats};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use num_enum::IntoPrimitive;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Spacing};
use ratatui::macros::{horizontal, vertical};
use ratatui::style::Stylize;
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, BorderType, Clear, StatefulWidget, Widget};
use strum::{Display, FromRepr, VariantArray};

mod uop_list;

pub mod decode;
pub mod fetch;
pub mod mem;
pub mod pipelines;
pub mod predecode;
pub mod regs;
pub mod rob;

pub struct Dashboard {
    fetch: Fetch,
    predecode: Predecode,
    decode: Decode,
    rob: Rob,
    pipelines: Pipelines,
    mem: MemSubsystem,
    int_regs: IntRegs,
    focus: Focus,
}

impl Dashboard {
    pub fn new(stats: Stats, config: &Config) -> Self {
        let Stats {
            addr_gen,
            fetch,
            predecode,
            decode,
            rob,
            pipelines,
            mem,
            csrf: _,
            mode: _,
        } = stats;

        Self {
            fetch: Fetch::new(addr_gen, fetch),
            predecode: Predecode::new(predecode),
            decode: Decode::new(decode),
            rob: Rob::new(rob),
            int_regs: IntRegs::new(pipelines.int.rat),
            pipelines: Pipelines::new(pipelines, &config.pipelines),
            mem: MemSubsystem::new(mem, &config.mem),
            focus: Default::default(),
        }
    }

    pub fn update(&mut self, stats: Stats) {
        let Stats {
            addr_gen,
            fetch,
            predecode,
            decode,
            rob,
            pipelines,
            mem,
            csrf: _,
            mode: _,
        } = stats;

        self.fetch.update(addr_gen, fetch);
        self.predecode.update(predecode);
        self.decode.update(decode);
        self.rob.update(rob);
        self.int_regs.update(pipelines.int.rat);
        self.pipelines.update(pipelines);
        self.mem.update(mem);
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> EventResult {
        self.focus = match event {
            key_event!(KeyCode::Enter) => return EventResult::Tick,
            key_event!(KeyCode::Up, KeyModifiers::CONTROL) => self.focus.up(),
            key_event!(KeyCode::Down, KeyModifiers::CONTROL) => self.focus.down(),
            key_event!(KeyCode::Left, KeyModifiers::CONTROL) => self.focus.left(),
            key_event!(KeyCode::Right, KeyModifiers::CONTROL) => self.focus.right(),
            _ => {
                match self.focus {
                    Focus::Fetch => self.fetch.handle_event(event),
                    Focus::Predecode => self.predecode.handle_event(event),
                    Focus::Decode => self.decode.handle_event(event),
                    Focus::Rob => self.rob.handle_event(event),
                    Focus::Pipelines => self.pipelines.handle_event(event),
                    Focus::Mem => self.mem.handle_event(event),
                    Focus::IntRegs => self.int_regs.handle_event(event),
                };
                return EventResult::Ok;
            }
        };

        EventResult::Ok
    }
}

impl StatefulWidget for &Dashboard {
    type State = NavInfoBuilder;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.extend([("Enter", "Tick"), ("Ctrl + ↑↓←→", "Change focus")]);
        let mut state = Some(state);

        let [area, regs_area] = vertical![*=1, ==6].areas(area);
        let [area, rob_area] = horizontal![*=1, ==25%].areas(area);
        let [area, middle_area] = vertical![*=1, *=2].areas(area);
        let [pipelines_area, mem_area] = horizontal![*=2, *=1].areas(middle_area);
        let [fetch_area, predecode_area, decode_area] = horizontal![*=1, *=1, *=1]
            .spacing(Spacing::Overlap(1))
            .areas(area);

        let base_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .merge_borders(MergeStrategy::Fuzzy);
        macro_rules! stage_blocks {
            ($( $stage:ident($state:tt, $fallback_title:literal $(, $title:expr )*) ),*) => {{
                ::paste::paste! {
                    stage_blocks![$( $stage{self.[<$stage:lower>]}($state, $fallback_title $(, $title)*) ),*];
                }
            }};
            ($( $stage:tt{$accessor:expr}($state:tt, $fallback_title:expr $(, $title:expr )*) ),*) => {
            $({
                ::paste::paste! {
                    let mut [<$stage:lower _block>] = base_block.clone().title_top(
                        match base_block.inner([<$stage:lower _area>]).width as usize {
                            $( x if x >= $title.len() => $title, )*
                            _ => $fallback_title,
                        }
                    );
                    let mut focused = state.take_if(|_| matches!(self.focus, Focus::$state));
                    if focused.is_some() {
                        [<$stage:lower _block>] = [<$stage:lower _block>].cyan();
                    }
                    let area = [<$stage:lower _block>].inner([<$stage:lower _area>]);
                    [<$stage:lower _block>].render([<$stage:lower _area>], buf);
                    Clear.render(area, buf);
                    $accessor.render(area, buf, &mut focused);
                    if focused.is_some() {
                        state = focused;
                    }
                }
            })*
            };
        }

        stage_blocks![
            Fetch(Fetch, "Fetch"),
            Predecode(Predecode, "Predecode"),
            Decode(Decode, "Decode"),
            Rob(Rob, "ROB", "Reorder Buffer"),
            Pipelines(Pipelines, "Pipelines")
        ];
        {
            let mut focused = state.take_if(|_| matches!(self.focus, Focus::Mem));
            self.mem.render(mem_area, buf, &mut focused);
            if focused.is_some() {
                state = focused;
            }
        }
        stage_blocks![Regs{self.int_regs}(IntRegs, "Regs", "Integer Registers", "Int Registers", "Int Regs")];
        let _ = state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventResult {
    Ok,
    Tick,
}

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Default, VariantArray, FromRepr, IntoPrimitive,
)]
#[repr(u8)]
enum Focus {
    #[default]
    Fetch,
    Predecode,
    Decode,
    Rob,
    Pipelines,
    Mem,
    IntRegs,
}

impl Focus {
    pub fn up(self) -> Self {
        match self {
            Self::Fetch | Self::Predecode | Self::Decode | Self::Rob => Self::IntRegs,
            Self::Pipelines => Self::Fetch,
            Self::Mem => Self::Decode,
            Self::IntRegs => Self::Pipelines,
        }
    }

    pub fn down(self) -> Self {
        match self {
            Self::Fetch | Self::Predecode => Self::Pipelines,
            Self::Decode => Self::Mem,
            Self::Rob | Self::Pipelines | Self::Mem => Self::IntRegs,
            Self::IntRegs => Self::Fetch,
        }
    }

    pub fn left(self) -> Self {
        match self {
            Self::Fetch | Self::Pipelines => Self::Rob,
            Self::Predecode => Self::Fetch,
            Self::Decode => Self::Predecode,
            Self::Rob => Self::Mem,
            Self::Mem => Self::Pipelines,
            Self::IntRegs => Self::IntRegs,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::Fetch => Self::Predecode,
            Self::Predecode => Self::Decode,
            Self::Decode | Self::Mem => Self::Rob,
            Self::Rob => Self::Pipelines,
            Self::Pipelines => Self::Mem,
            Self::IntRegs => Self::IntRegs,
        }
    }
}

pub type Focused<'a> = Option<&'a mut NavInfoBuilder>;

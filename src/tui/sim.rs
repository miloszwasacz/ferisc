use self::dashboard::Dashboard;
use self::nav::NavInfoBuilder;
use crate::core::{Config, Core, Stats, mem::DirectHierarchy};
use crate::mem::Dram;
use crate::seq::Sequential;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use num_enum::IntoPrimitive;
use ratatui::layout::Offset;
use ratatui::macros::vertical;
use ratatui::prelude::*;
use ratatui::symbols;
use ratatui::text::ToLine;
use ratatui::widgets::{Block, BorderType, Tabs};
use std::sync::{Arc, RwLock};
use strum::{Display, FromRepr, VariantArray};

mod dashboard;
mod nav;

pub struct Sim {
    // Simulation
    //TODO Extract the simulation to run on a separate thread
    core: Core<DirectHierarchy>,
    mem: Arc<RwLock<Dram>>,

    // UI
    tabs: TabState,
    dashboard: Dashboard,
}

impl Sim {
    #[allow(clippy::new_without_default)] //TODO Create from config
    pub fn new() -> Self {
        //TODO Proper error handling
        let elf_data = include_bytes!("../../simple_rv64");
        let elf = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(elf_data).unwrap();
        let entry = elf.ehdr.e_entry;
        let mem = Arc::new(RwLock::new(Dram::new(1024 * 1024 * 1024, elf, elf_data)));
        let core = make_core(entry, mem.clone());

        Self {
            tabs: Default::default(),
            dashboard: Dashboard::new(core.stats()),

            core,
            mem,
        }
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> EventResult {
        match event {
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => EventResult::Quit,
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.tabs = self.tabs.next();
                EventResult::Ok
            }
            KeyEvent {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::SHIFT,
                ..
            } => {
                self.tabs = self.tabs.prev();
                EventResult::Ok
            }
            event => match self.tabs {
                TabState::Dashboard => match self.dashboard.handle_event(event) {
                    dashboard::EventResult::Ok => EventResult::Ok,
                    dashboard::EventResult::Tick => {
                        self.core.tick();
                        self.core.finish_cycle();
                        self.mem.write().unwrap().finish_cycle();

                        self.dashboard = Dashboard::new(self.core.stats());
                        EventResult::Ok
                    }
                },
                TabState::Csrf => todo!(),
            },
        }
    }
}

impl Widget for &Sim {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [tabs_area, nav_area] = vertical![*=1, ==1].areas(area);

        let mut nav = NavInfoBuilder::new();
        nav.push_back(("Esc", "Quit"));

        render_tabs(self.core.stats(), tabs_area, buf, self.tabs, &mut nav);

        nav.build().render(nav_area, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventResult {
    Ok,
    Quit,
    //TODO Err
}

//#region Tabs

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Default, VariantArray, FromRepr, IntoPrimitive,
)]
#[repr(u8)]
enum TabState {
    #[default]
    Dashboard = 0,
    #[strum(to_string = "CSRs")]
    Csrf,
}

impl TabState {
    pub fn next(self) -> Self {
        const FIRST: TabState = TabState::VARIANTS.first().copied().unwrap();

        Self::from_repr(u8::from(self).wrapping_add(1)).unwrap_or(FIRST)
    }

    pub fn prev(self) -> Self {
        const LAST: TabState = TabState::VARIANTS.last().copied().unwrap();

        Self::from_repr(u8::from(self).wrapping_sub(1)).unwrap_or(LAST)
    }
}

fn render_tabs(
    stats: Stats,
    area: Rect,
    buf: &mut Buffer,
    state: TabState,
    nav: &mut NavInfoBuilder,
) {
    nav.push_back(("Tab", "Change tab"));

    let block = Block::bordered().border_type(BorderType::Rounded);
    match state {
        TabState::Dashboard => Dashboard::new(stats).render(block.inner(area), buf, nav),
        TabState::Csrf => {
            //TODO CSRs
        }
    }
    block.render(area, buf);

    Tabs::new(TabState::VARIANTS.iter().map(|v| v.to_line()))
        .select(state as usize)
        .divider(symbols::line::HORIZONTAL)
        .padding("", "")
        .render(area + Offset::new(2, 0), buf);
}

//#endregion

//TODO Remove this
fn make_core(entrypoint: crate::mem::Addr, mem: Arc<RwLock<Dram>>) -> Core<DirectHierarchy> {
    use crate::core::{
        ee::{eu, pipeline, rob, rs},
        fe::{decode, fetch},
        mem,
    };
    use std::num::{NonZeroU8, NonZeroUsize};

    Core::new(
        Config {
            entrypoint,
            fetch: fetch::Config {
                packet_width: NonZeroU8::new(4 * size_of::<u32>() as u8).unwrap(),
            },
            decode: decode::Config {
                width: NonZeroU8::new(1).unwrap(),
                queue_size: None,
            },
            rob: rob::Config {
                size: NonZeroUsize::new(100).unwrap(),
                commit_width: NonZeroU8::new(1).unwrap(),
                flush_width: None,
            },
            pipelines: pipeline::Configs {
                int: pipeline::Config {
                    prf_size: NonZeroUsize::new(200).unwrap(),
                    lines: [rs::Config {
                        size: NonZeroUsize::new(10).unwrap(),
                    }]
                    .map(|rs| (rs, eu::Config {}))
                    .repeat(5)
                    .into_boxed_slice(),
                },
            },
            mem: mem::Config {
                ldq_size: NonZeroUsize::new(10).unwrap(),
                stq_size: NonZeroUsize::new(10).unwrap(),
            },
        },
        || DirectHierarchy::new(mem),
    )
}

use self::dashboard::Dashboard;
use self::nav::NavInfoBuilder;
use crate::core::{Config, Core, mem::DirectHierarchy, reg::int::IntVal};
use crate::mem::{Addr, Dram};
use crate::seq::Sequential;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::vertical;
use ratatui::symbols;
use ratatui::text::ToLine;
use ratatui::widgets::{StatefulWidget, Tabs, Widget};
use std::fmt;
use std::sync::{Arc, RwLock};
use strum::{Display, VariantArray};

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
        let config = make_core_config(entry);
        let core = Core::new(&config, || DirectHierarchy::new(mem.clone()));

        Self {
            tabs: Default::default(),
            dashboard: Dashboard::new(core.stats(), &config),

            core,
            mem,
        }
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> EventResult {
        match event {
            key_event!(KeyCode::Esc) => EventResult::Quit,
            key_event!(KeyCode::Tab) => {
                self.tabs = self.tabs.next();
                EventResult::Ok
            }
            key_event!(KeyCode::BackTab, KeyModifiers::SHIFT) => {
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

                        self.dashboard.update(self.core.stats());
                        EventResult::Ok
                    }
                },
                TabState::Csrf => todo!(),
            },
        }
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer, nav: &mut NavInfoBuilder) {
        nav.push_back(("Tab", "Change tab"));

        let content_area = Rect {
            y: area.y + 2,
            height: area.height - 2,
            ..area
        };
        match self.tabs {
            TabState::Dashboard => self.dashboard.render(content_area, buf, nav),
            TabState::Csrf => {
                //TODO CSRs
            }
        }

        //TODO Prettier tabs
        Tabs::new(TabState::VARIANTS.iter().map(|v| v.to_line()))
            .select(self.tabs as usize)
            .divider(symbols::DOT)
            .render(area, buf);
    }
}

impl Widget for &Sim {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [tabs_area, nav_area] = vertical![*=1, ==1].areas(area);

        let mut nav = NavInfoBuilder::new();
        nav.push_back(("Esc", "Quit"));
        self.render_tabs(tabs_area, buf, &mut nav);
        nav.build().render(nav_area, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventResult {
    Ok,
    Quit,
    //TODO Err
}

focus_state! {
    #[repr(u8)]
    #[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Default)]
    enum TabState {
        #[default]
        Dashboard = 0,
        #[strum(to_string = "CSRs")]
        Csrf,
    }
}

//#region AddrView

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddrView(pub Addr);

impl AddrView {
    /// The minimal width of the hexadecimal representation of the address
    /// (excluding the _0x_ prefix).
    pub const PRETTY_WIDTH: usize = 7;
}

impl fmt::Display for AddrView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            write!(f, "0x{:0width$x}", self.0, width = f.width().unwrap_or(0))
        } else {
            write!(
                f,
                "{:0width$x}",
                self.0,
                width = f
                    .width()
                    .map(|w| w.max(Self::PRETTY_WIDTH))
                    .unwrap_or(Self::PRETTY_WIDTH)
            )
        }
    }
}

impl IntVal {
    pub fn addr_view(&self) -> AddrView {
        AddrView(self.addr())
    }
}

//#endregion

//TODO Remove this
fn make_core_config(entrypoint: Addr) -> Config {
    use crate::core::{
        ee::{eu, pipeline, rob, rs},
        fe::{decode, fetch},
        mem,
    };
    use std::num::{NonZeroU8, NonZeroUsize};

    Config {
        entrypoint,
        fetch: fetch::Config {
            packet_width: NonZeroU8::new(4 * size_of::<u32>() as u8).unwrap(),
        },
        decode: decode::Config {
            width: NonZeroU8::new(4).unwrap(),
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
    }
}

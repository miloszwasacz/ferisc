use ratatui::DefaultTerminal;

macro_rules! key_event {
    ($code:pat) => {
        ::crossterm::event::KeyEvent {
            code: $code,
            modifiers: ::crossterm::event::KeyModifiers::NONE,
            ..
        }
    };
    ($code:pat, $modifiers:pat) => {
        ::crossterm::event::KeyEvent {
            code: $code,
            modifiers: $modifiers,
            ..
        }
    };
}

macro_rules! focus_state {
    (
        $( #[doc = $doc:literal] )*
        #[repr($repr:tt)]
        $( #[$meta1:meta] )*
        $vis:vis enum $name:ident {
            $( $variants:tt )+
        }
    ) => {
        $( #[doc = $doc] )*
        $( #[$meta1] )*
        #[repr($repr)]
        #[derive(::strum::VariantArray, ::strum::FromRepr, ::num_enum::IntoPrimitive)]
        $vis enum $name {
            $( $variants )+
        }

        impl $name {
            pub fn next(self) -> Self {
                use ::strum::VariantArray;
                const FIRST: $name = $name::VARIANTS.first().copied().unwrap();

                Self::from_repr($repr::from(self).wrapping_add(1)).unwrap_or(FIRST)
            }

            pub fn prev(self) -> Self {
                use ::strum::VariantArray;
                const LAST: $name = $name::VARIANTS.last().copied().unwrap();

                Self::from_repr($repr::from(self).wrapping_sub(1)).unwrap_or(LAST)
            }
        }
    };
}

pub mod components;
pub mod focus;
pub mod sim;

pub fn run() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut sim = sim::Sim::new();
    loop {
        terminal.draw(|frame| frame.render_widget(&sim, frame.area()))?;
        match crossterm::event::read()?
            .as_key_event()
            .map(|event| sim.handle_event(event))
            .unwrap_or(sim::EventResult::Ok)
        {
            sim::EventResult::Ok => {}
            sim::EventResult::Quit => break Ok(()),
        }
    }
}

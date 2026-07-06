use ratatui::DefaultTerminal;

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

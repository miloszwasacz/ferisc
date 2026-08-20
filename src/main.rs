fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    cfg_select! {
        feature = "tui" => ferisc::tui::run(),
        _ => Ok(()),
    }
}

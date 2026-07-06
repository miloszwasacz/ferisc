fn main() -> color_eyre::Result<()> {
    cfg_select! {
        feature = "tui" => ferisc::tui::run(),
        _ => Ok(())
    }
}

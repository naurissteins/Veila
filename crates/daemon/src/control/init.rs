use anyhow::Result;

pub(super) fn init_config(
    config_path: Option<&std::path::Path>,
    theme: Option<&str>,
    force: bool,
) -> Result<()> {
    let theme = theme.unwrap_or("default");
    let written_path = veila_common::config::init_config(config_path, theme, force)?;

    println!("initialized=true");
    println!("config={}", written_path.display());
    println!("theme={theme}");
    println!("force={force}");

    Ok(())
}

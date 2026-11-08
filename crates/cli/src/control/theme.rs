use anyhow::Result;

pub(super) fn print_available_themes() -> Result<()> {
    for theme in veila_common::config::bundled_theme_names()? {
        println!("{theme}");
    }
    Ok(())
}

pub(super) fn print_current_theme(config_path: Option<&std::path::Path>) -> Result<()> {
    let theme = veila_common::config::active_theme_name(config_path)?;
    let source = veila_common::config::active_theme_source_path(config_path)?;

    println!("theme={}", theme.as_deref().unwrap_or("none"));
    println!(
        "source={}",
        source
            .as_deref()
            .map(|path| path.display().to_string())
            .as_deref()
            .unwrap_or("none")
    );

    Ok(())
}

pub(super) fn print_theme_source(theme: &str, config_path: Option<&std::path::Path>) -> Result<()> {
    let (path, raw) = veila_common::config::read_theme_source(config_path, theme)?;
    println!("theme={theme}");
    println!("source={}", path.display());
    println!();
    print!("{raw}");
    Ok(())
}

pub(super) fn set_theme_and_reload(
    theme: &str,
    config_path: Option<&std::path::Path>,
) -> Result<()> {
    let path = veila_common::config::set_theme_in_config(config_path, theme)?;
    println!("theme={theme}\nconfig={}", path.display());
    super::runtime::reload_config()
}
pub(super) fn unset_theme_and_reload(config_path: Option<&std::path::Path>) -> Result<()> {
    let (path, changed) = veila_common::config::unset_theme_in_config(config_path)?;
    println!("config={}\ntheme_removed={changed}", path.display());
    if changed {
        super::runtime::reload_config()?;
    }
    Ok(())
}

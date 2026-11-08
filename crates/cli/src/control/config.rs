use anyhow::{Result, bail};

pub(super) fn print_config_validation(config_path: Option<&std::path::Path>) -> Result<()> {
    let report = match veila_common::AppConfig::validate(config_path) {
        Ok(report) => report,
        Err(error) => {
            println!("config_valid=false");
            println!("config_error={error}");
            return Err(error.into());
        }
    };

    println!("config_valid={}", report.is_valid());
    println!(
        "config={}",
        report
            .config_path
            .as_deref()
            .map(|path| path.display().to_string())
            .as_deref()
            .unwrap_or("defaults")
    );
    println!("sources={}", report.sources.len());
    for (index, source) in report.sources.iter().enumerate() {
        println!("source.{index}.kind={}", source.kind.as_str());
        println!("source.{index}.path={}", source.path.display());
    }
    println!("issues={}", report.issues.len());
    for (index, issue) in report.issues.iter().enumerate() {
        println!("issue.{index}.source={}", issue.source.as_str());
        println!("issue.{index}.file={}", issue.path.display());
        println!("issue.{index}.key={}", issue.key_path);
        println!("issue.{index}.message={}", issue.message);
    }

    if report.is_valid() {
        Ok(())
    } else {
        bail!("config validation failed")
    }
}

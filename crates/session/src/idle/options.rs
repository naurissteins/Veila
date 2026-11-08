use anyhow::{Context, Result, bail};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct IdleOptions {
    pub config: Option<PathBuf>,
    pub session_id: Option<String>,
    pub help: bool,
}

impl IdleOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut result = Self::default();
        let mut args = args.into_iter().skip(1);
        while let Some(arg) = args.next() {
            if matches!(arg.as_str(), "--help" | "-h") {
                result.help = true;
                continue;
            }
            let (key, value) = match arg.split_once('=') {
                Some((key, value)) => (key.to_owned(), value.to_owned()),
                None => (arg, args.next().context("option requires a value")?),
            };
            if value.is_empty() || value.starts_with("--") {
                bail!("{key} requires a value");
            }
            match key.as_str() {
                "--config" => result.config = Some(value.into()),
                "--session-id" => result.session_id = Some(value),
                _ => bail!("unknown idle watcher argument: {key}"),
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<IdleOptions> {
        IdleOptions::parse(
            std::iter::once("veila")
                .chain(args.iter().copied())
                .map(str::to_owned),
        )
    }
    #[test]
    fn accepts_config_and_explicit_session_but_rejects_daemon_options() {
        let options = parse(&["--config=a", "--session-id", "2"]).expect("options");
        assert_eq!(options.config, Some("a".into()));
        assert_eq!(options.session_id.as_deref(), Some("2"));
        for args in [
            &["--config"][..],
            &["--session-id="],
            &["--config", "--help"],
            &["--daemonize"],
        ] {
            assert!(parse(args).is_err());
        }
    }
}

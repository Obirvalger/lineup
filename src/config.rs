use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

use anyhow::{Context, Result};
use log::LevelFilter;
use serde::Deserialize;

pub static CONFIG: LazyLock<Config> =
    LazyLock::new(|| CONFIG_INNER.get().expect("Config should be initialized").to_owned());
static CONFIG_INNER: OnceLock<Config> = OnceLock::new();

pub fn init() -> Result<()> {
    let config = Config::new()?;
    CONFIG_INNER.get_or_init(|| config);

    Ok(())
}

fn default_install_embedded_modules() -> bool {
    true
}

fn default_clean() -> bool {
    true
}

fn default_log_level() -> LevelFilter {
    LevelFilter::Info
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: LevelFilter,
    #[serde(default = "default_install_embedded_modules")]
    pub install_embedded_modules: bool,
    #[serde(default = "default_clean")]
    pub clean: bool,
}

fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy().to_string();
    PathBuf::from(shellexpand::tilde(&s).to_string())
}

pub fn config_dir() -> PathBuf {
    let home_config_dir = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| "~/.config".to_string());
    expand_tilde(&PathBuf::from(home_config_dir)).join("lineup")
}

impl Config {
    pub fn new() -> Result<Config> {
        let config_path = config_dir().join("config.toml");
        let config_str = &fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config `{}`", &config_path.display()))?;

        let config: Config = toml::from_str(config_str)
            .with_context(|| format!("Failed to parse config `{}`", &config_path.display()))?;

        Ok(config)
    }
}

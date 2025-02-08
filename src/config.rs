use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, OnceLock};

use anyhow::{Context, Result};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

use crate::files::install_file;
use crate::task_type::CmdOutput;

pub static CONFIG: LazyLock<Config> =
    LazyLock::new(|| CONFIG_INNER.get().expect("Config should be initialized").to_owned());
static CONFIG_INNER: OnceLock<Config> = OnceLock::new();
static CONFIG_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn config_initialized() -> bool {
    CONFIG_INITIALIZED.load(Ordering::Relaxed)
}

pub fn init() -> Result<()> {
    if !config_dir().join("config.toml").exists() {
        install_file("configs/config.toml", &config_dir())?;
    }

    let config = Config::new()?;
    CONFIG_INNER.get_or_init(|| config);
    CONFIG_INITIALIZED.store(true, Ordering::Relaxed);

    Ok(())
}

fn default_command_task_check() -> bool {
    true
}

fn default_command_stdout() -> CmdOutput {
    CmdOutput { log: LevelFilter::Trace, print: false }
}

fn default_command_stderr() -> CmdOutput {
    CmdOutput { log: LevelFilter::Warn, print: false }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct CommandTask {
    #[serde(default = "default_command_task_check")]
    pub check: bool,
    #[serde(default = "default_command_stdout")]
    pub stdout: CmdOutput,
    #[serde(default = "default_command_stderr")]
    pub stderr: CmdOutput,
}

impl Default for CommandTask {
    fn default() -> Self {
        Self {
            check: default_command_task_check(),
            stdout: default_command_stdout(),
            stderr: default_command_stderr(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(default)]
    pub command: CommandTask,
}

fn default_error_backtrace() -> bool {
    true
}

fn default_error_context() -> bool {
    true
}

fn default_error_context_lines() -> usize {
    10
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Error {
    #[serde(default = "default_error_backtrace")]
    pub backtrace: bool,
    #[serde(default = "default_error_context")]
    pub context: bool,
    #[serde(default = "default_error_context_lines")]
    pub context_lines: usize,
}

impl Default for Error {
    fn default() -> Self {
        Self {
            backtrace: default_error_backtrace(),
            context: default_error_context(),
            context_lines: default_error_context_lines(),
        }
    }
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: LevelFilter,
    #[serde(default = "default_install_embedded_modules")]
    pub install_embedded_modules: bool,
    #[serde(default = "default_clean")]
    pub clean: bool,
    #[serde(default)]
    pub task: Task,
    #[serde(default)]
    pub error: Error,
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

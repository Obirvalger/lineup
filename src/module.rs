use std::path::{Path, PathBuf};

use crate::config::config_dir;

pub fn resolve(module: &Path, dir: &Path) -> PathBuf {
    if module.is_absolute() {
        module.to_owned()
    } else if module.starts_with(".") || module.starts_with("..") {
        dir.join(module)
    } else {
        config_dir().join("modules").join(module).with_extension("toml")
    }
}

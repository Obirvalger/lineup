use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use file_lock::{FileLock, FileOptions};
use rust_embed::RustEmbed;

use crate::config::config_dir;

#[derive(RustEmbed)]
#[folder = "files/configs"]
#[prefix = "configs/"]
struct AssetConfigs;

#[derive(RustEmbed)]
#[folder = "files/modules"]
#[prefix = "modules/"]
struct AssetModules;

#[derive(RustEmbed)]
#[folder = "files"]
struct AssetAllFiles;

fn lock_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let options = FileOptions::new().create(true).truncate(true).write(true);
    let block = true;
    if let Ok(mut filelock) = FileLock::lock(path.as_ref(), block, options) {
        filelock.file.write_all(contents.as_ref())?;
    }

    Ok(())
}

fn install_file<S: AsRef<str>>(filename: S, directory: &Path) -> Result<()> {
    let filename = filename.as_ref();
    let mut basename = filename.to_owned();
    let path = Path::new(filename);
    if let Some(parent) = path.parent() {
        if parent != Path::new("") {
            basename = path
                .file_name()
                .expect("trying to install file with bad filename")
                .to_string_lossy()
                .to_string();
        }
    }
    fs::create_dir_all(directory)?;

    let file = directory.join(basename);
    let content = AssetAllFiles::get(filename).unwrap();
    lock_write(file, content.data)?;

    Ok(())
}

fn install_hier<S: AsRef<str>>(filename: S, directory: &Path) -> Result<()> {
    let filename = filename.as_ref();
    let mut basename = filename.to_owned();
    let path = Path::new(filename);
    let mut directory = directory.to_owned();
    if let Some(parent) = path.parent() {
        if parent != Path::new("") {
            directory = directory.join(parent);
            basename = path
                .file_name()
                .expect("trying to install file with bad filename")
                .to_string_lossy()
                .to_string();
        }
    }
    fs::create_dir_all(&directory)?;

    let file = directory.join(basename);
    let content = AssetAllFiles::get(filename).unwrap();
    lock_write(file, content.data)?;

    Ok(())
}

pub fn install_main_config() -> Result<()> {
    if !config_dir().join("config.toml").exists() {
        install_file("configs/config.toml", &config_dir())?;
    }

    Ok(())
}

pub fn install_modules() -> Result<()> {
    for file in AssetModules::iter() {
        install_hier(file, &config_dir())?;
    }

    Ok(())
}

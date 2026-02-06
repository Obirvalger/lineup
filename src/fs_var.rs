use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::error::Error;
use crate::tmpdir::TMPDIR;

static FS_VAR_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| FS_VAR_DIR_INNER.get().unwrap_or(&TMPDIR).to_path_buf().join("fs_vars"));
static FS_VAR_DIR_INNER: OnceLock<PathBuf> = OnceLock::new();

pub fn set_fs_var_dir<P: AsRef<Path>>(dir: P) {
    FS_VAR_DIR_INNER.get_or_init(|| dir.as_ref().to_path_buf());
}

pub fn cleanup() -> Result<()> {
    if FS_VAR_DIR.exists() {
        fs::remove_dir_all(&*FS_VAR_DIR).context("Cleanup fs var dir")?;
    }

    Ok(())
}

pub struct FsVar {
    name: String,
}

impl FsVar {
    pub fn new<S: AsRef<str>>(name: S) -> Result<Self> {
        let name = name.as_ref().to_string();
        if !name.chars().all(|c| char::is_alphanumeric(c) || c == '_') {
            bail!(Error::BadFsVar(name))
        }

        Ok(Self { name })
    }

    fn dir(&self) -> PathBuf {
        FS_VAR_DIR.join("simple")
    }

    fn path(&self) -> PathBuf {
        self.dir().join(&self.name)
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    pub fn read(&self) -> Result<Value> {
        let s = fs::read_to_string(self.path())
            .with_context(|| format!("reading fs var {}", &self.name))?;
        Ok(serde_json::from_str(&s)?)
    }

    fn ensure_dir(&self) -> Result<()> {
        let dir = self.dir();
        if !dir.exists() {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating dirs for fs var {}", &self.name))?
        };

        Ok(())
    }

    pub fn write(&self, value: &Value) -> Result<()> {
        self.ensure_dir()?;

        fs::write(self.path(), value.to_string())
            .with_context(|| format!("writing fs var {}", &self.name))
    }
}

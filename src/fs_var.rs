use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::error::Error;
use crate::tmpdir::TMPDIR;

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
        TMPDIR.join("fs_vars").join("simple")
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

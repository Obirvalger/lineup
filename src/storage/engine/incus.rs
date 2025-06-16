use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{bail, Result};
use cmd_lib::run_fun;

use crate::error::Error;
use crate::manifest::StorageEngineIncus as ManifestEngineIncus;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineIncus {
    pub pool: String,
    pub copy: Option<String>,
    incus_bin: String,
    is_setup: Arc<AtomicBool>,
}

impl EngineIncus {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_incus: &ManifestEngineIncus,
    ) -> Result<Self> {
        let manifest_engine_incus =
            manifest_engine_incus.render(context, "storage engine in manifest")?;
        let incus_bin = "incus".to_string();

        Ok(Self {
            pool: manifest_engine_incus.pool,
            copy: manifest_engine_incus.copy,
            incus_bin,
            is_setup: Arc::new(AtomicBool::new(false)),
        })
    }

    fn exists<S: AsRef<str>>(&self, volume: S) -> Result<bool> {
        let incus = &self.incus_bin;
        let volume = volume.as_ref();
        let pool = &self.pool;

        let exists = run_fun!($incus storage volume ls $pool -f json name=$volume type=custom)?;

        Ok(exists != "[]")
    }

    pub fn setup<S: AsRef<str>>(&self, volume: S) -> Result<()> {
        if self
            .is_setup
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        };

        let incus = &self.incus_bin;
        let volume = volume.as_ref();
        let pool = &self.pool;

        if let Some(from) = &self.copy {
            run_fun!($incus storage volume copy $pool/$from $pool/$volume -q)?;
        } else {
            run_fun!($incus storage volume create $pool $volume -q)?;
        }

        if self.exists(volume)? {
            Ok(())
        } else {
            bail!(Error::FailSetupIncusVolume(volume.to_string()))
        }
    }

    pub fn remove<S: AsRef<str>>(&self, volume: S) -> Result<()> {
        if !self.exists(volume.as_ref())? {
            return Ok(());
        }

        let incus = &self.incus_bin;
        let volume = volume.as_ref();
        let pool = &self.pool;

        run_fun!($incus storage volume delete $pool $volume -q)?;

        Ok(())
    }
}

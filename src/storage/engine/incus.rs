use anyhow::Result;
use cmd_lib::run_fun;

use crate::manifest::StorageEngineIncus as ManifestEngineIncus;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineIncus {
    pub pool: String,
    incus_bin: String,
}

impl EngineIncus {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_incus: &ManifestEngineIncus,
    ) -> Result<Self> {
        let manifest_engine_incus =
            manifest_engine_incus.render(context, "storage engine in manifest")?;
        let incus_bin = "incus".to_string();

        Ok(Self { pool: manifest_engine_incus.pool, incus_bin })
    }

    fn exists<S: AsRef<str>>(&self, volume: S) -> Result<bool> {
        let incus = &self.incus_bin;
        let volume = volume.as_ref();
        let pool = &self.pool;

        let exists = run_fun!($incus storage volume ls $pool -f json name=$volume type=custom)?;

        Ok(exists != "[]")
    }

    pub fn setup<S: AsRef<str>>(&self, volume: S) -> Result<()> {
        if self.exists(volume.as_ref())? {
            return Ok(());
        }

        let incus = &self.incus_bin;
        let volume = volume.as_ref();
        let pool = &self.pool;

        run_fun!($incus storage volume create $pool $volume -q)?;

        Ok(())
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

use anyhow::Result;

use crate::manifest::Storages as ManifestStorages;
use crate::render::Render;
use crate::storage::engine::Engine;
use crate::template::Context;

mod engine;

#[derive(Clone, Debug)]
pub struct Storage {
    pub volume: String,
    engine: Engine,
}

impl Storage {
    pub fn from_manifest_storages(
        manifest_storages: &ManifestStorages,
        context: &Context,
    ) -> Result<Vec<Self>> {
        let mut storages = Vec::with_capacity(manifest_storages.len());
        for (volume, manifest_storage) in manifest_storages {
            let volume = volume.render(context, "storage in manifest")?;
            let storage = Storage {
                volume,
                engine: Engine::from_manifest_engine(context, &manifest_storage.engine)?,
            };

            storages.push(storage);
        }

        Ok(storages)
    }

    pub fn setup(&self) -> Result<()> {
        self.engine.setup(&self.volume)
    }

    pub fn remove(&self) -> Result<()> {
        self.engine.remove(&self.volume)
    }
}

use std::collections::BTreeMap;

use anyhow::Result;

use crate::manifest::Storages as ManifestStorages;
use crate::render::Render;
use crate::storage::engine::Engine;
use crate::template::Context;

mod engine;

pub type Storages = BTreeMap<String, Storage>;

#[derive(Clone, Debug)]
pub struct Storage {
    pub volume: String,
    engine: Engine,
}

impl Storage {
    pub fn from_manifest_storages(
        manifest_storages: &ManifestStorages,
        context: &Context,
    ) -> Result<BTreeMap<String, Self>> {
        let mut storages = BTreeMap::new();
        let mut context = context.to_owned();
        for (volume, manifest_storage) in manifest_storages {
            let items = manifest_storage
                .items
                .as_ref()
                .map(|i| i.list(&context))
                .transpose()?
                .unwrap_or_else(|| vec!["".to_string()]);

            for item in items {
                context.insert("item", &item);
                let volume = volume.render(&context, "storage in manifest")?;
                let storage = Storage {
                    volume: volume.to_string(),
                    engine: Engine::from_manifest_engine(&context, &manifest_storage.engine)?,
                };

                storages.insert(volume, storage);
            }
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

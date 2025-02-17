use anyhow::Result;

use crate::manifest::StorageEngine as ManifestEngine;
use crate::storage::engine::incus::EngineIncus;
use crate::template::Context;

mod incus;

#[derive(Clone, Debug)]
pub enum Engine {
    Incus(EngineIncus),
}

impl Engine {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine: &ManifestEngine,
    ) -> Result<Engine> {
        let engine = match manifest_engine {
            ManifestEngine::Incus(manifest_engine_incus) => {
                Engine::Incus(EngineIncus::from_manifest_engine(context, manifest_engine_incus)?)
            }
        };

        Ok(engine)
    }

    pub fn setup<S: AsRef<str>>(&self, volume: S) -> Result<()> {
        match self {
            Engine::Incus(engine) => engine.setup(volume),
        }
    }

    pub fn remove<S: AsRef<str>>(&self, volume: S) -> Result<()> {
        match self {
            Engine::Incus(engine) => engine.remove(volume),
        }
    }
}

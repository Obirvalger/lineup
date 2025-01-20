use anyhow::Result;

use crate::manifest::Networks as ManifestNetworks;
use crate::network::engine::Engine;
use crate::render::Render;
use crate::template::Context;

mod engine;

#[derive(Clone, Debug)]
pub struct Network {
    pub name: String,
    engine: Engine,
}

impl Network {
    pub fn from_manifest_networks(
        manifest_networks: &ManifestNetworks,
        context: &Context,
    ) -> Result<Vec<Self>> {
        let mut networks = Vec::with_capacity(manifest_networks.len());
        for (name, manifest_network) in manifest_networks {
            let name = name.render(context, "network in manifest")?;
            let network = Network {
                name,
                engine: Engine::from_manifest_engine(context, &manifest_network.engine)?,
            };

            networks.push(network);
        }

        Ok(networks)
    }

    pub fn setup(&self) -> Result<()> {
        self.engine.setup(&self.name)
    }

    pub fn remove(&self) -> Result<()> {
        self.engine.remove(&self.name)
    }
}

use anyhow::Context as AnyhowContext;
use anyhow::Result;
use cmd_lib::run_fun;
use serde::Deserialize;

use crate::manifest::NetworkEngineIncus as ManifestEngineIncus;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineIncus {
    pub address: String,
    pub nat: bool,
    incus_bin: String,
}

impl EngineIncus {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_incus: &ManifestEngineIncus,
    ) -> Result<Self> {
        let manifest_engine_incus =
            manifest_engine_incus.render(context, "network engine in manifest")?;
        let incus_bin = "incus".to_string();

        Ok(Self {
            address: manifest_engine_incus.address,
            nat: manifest_engine_incus.nat,
            incus_bin,
        })
    }

    fn exists<S: AsRef<str>>(&self, name: S) -> Result<bool> {
        let incus = &self.incus_bin;
        let name = name.as_ref();

        let networks_str = run_fun!($incus network ls -f json)?;
        let networks: Vec<NetworkListElem> =
            serde_json::from_str(&networks_str).context("Incus network list")?;
        for network in networks {
            if network.name == name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn setup<S: AsRef<str>>(&self, name: S) -> Result<()> {
        if self.exists(name.as_ref())? {
            return Ok(());
        }

        let incus = &self.incus_bin;
        let name = name.as_ref();

        let mut options = vec!["-q".to_string()];

        options.push(format!("ipv4.address={}", &self.address));
        options.push(format!("ipv4.nat={}", &self.nat));

        run_fun!($incus network create $name $[options])?;

        Ok(())
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        if !self.exists(name.as_ref())? {
            return Ok(());
        }

        let incus = &self.incus_bin;
        let name = name.as_ref();

        run_fun!($incus network delete $name -q)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct NetworkListElem {
    name: String,
}

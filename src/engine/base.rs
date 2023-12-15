use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::render::Render;
use crate::template::Context;

fn default_engine_base_setup() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
/// Store base engine fields common to all engines
pub struct EngineBase {
    pub name: Option<String>,
    #[serde(default = "default_engine_base_setup")]
    pub setup: bool,
}

impl Render for EngineBase {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let name = self.name.render(context, format!("name in {}", place.as_ref()))?;
        Ok(EngineBase { name, ..self.to_owned() })
    }
}

impl Default for EngineBase {
    fn default() -> EngineBase {
        EngineBase { name: None, setup: default_engine_base_setup() }
    }
}

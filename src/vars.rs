use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tera::Context;

use crate::render::Render;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Vars {
    #[serde(flatten)]
    map: BTreeMap<String, Value>,
}

impl Vars {
    pub fn new(map: BTreeMap<String, Value>) -> Self {
        Self { map }
    }

    pub fn extend(&mut self, other: Self) {
        self.map.extend(other.map);
    }

    pub fn into_map(self) -> BTreeMap<String, Value> {
        self.map
    }

    pub fn context(&self) -> Result<Context> {
        let context = Context::from_serialize(self)?;

        Ok(context)
    }
}

impl Render for Vars {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let mut new_map = BTreeMap::new();
        for (name, value) in &self.map {
            new_map.insert(
                name.to_string(),
                value.render(context, format!("variables in {}", place.as_ref()))?,
            );
        }

        Ok(Self::new(new_map))
    }
}

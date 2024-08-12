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
    pub map: BTreeMap<String, Value>,
}

fn render_value<S: AsRef<str>>(value: &mut Value, context: &Context, place: S) -> Result<()> {
    match value {
        Value::String(s) => *s = s.render(context, format!("variables in {}", place.as_ref()))?,
        Value::Object(m) => {
            for (_, v) in m.iter_mut() {
                render_value(v, context, place.as_ref())?;
            }
        }
        Value::Array(a) => {
            for v in a.iter_mut() {
                render_value(v, context, place.as_ref())?;
            }
        }
        _ => {}
    }

    Ok(())
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
            let mut value = value.to_owned();
            render_value(&mut value, context, place.as_ref())?;
            new_map.insert(name.to_string(), value);
        }

        Ok(Self::new(new_map))
    }
}

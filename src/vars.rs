use std::collections::BTreeMap;
use std::convert::From;

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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", from = "Vec<Vars>", into = "Vec<Vars>")]
pub struct Maps {
    maps: Vec<Vars>,
    context: Context,
    place: String,
}

impl Maps {
    pub fn vars(self) -> Result<Vars> {
        let mut new_vars = Vars::new(BTreeMap::new());
        let mut context = self.context;

        for vars in self.maps {
            let vars = vars.render(&context, format!("ExtVars::vars in {}", &self.place))?;
            context.extend(vars.context()?);
            new_vars.extend(vars);
        }

        Ok(new_vars)
    }
}

impl From<Vec<Vars>> for Maps {
    fn from(maps: Vec<Vars>) -> Self {
        Self { maps, context: Context::new(), place: Default::default() }
    }
}

impl From<Maps> for Vec<Vars> {
    fn from(val: Maps) -> Self {
        val.maps
    }
}

impl Render for Maps {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let maps = self.maps.to_owned();
        let context = context.to_owned();
        let place = place.as_ref().to_string();

        Ok(Self { maps, context, place })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum ExtVars {
    Vars(Vars),
    Maps(Maps),
}

impl ExtVars {
    pub fn vars(self) -> Result<Vars> {
        match self {
            Self::Vars(vars) => Ok(vars),
            Self::Maps(maps) => Ok(maps.vars()?),
        }
    }
}

impl Default for ExtVars {
    fn default() -> Self {
        ExtVars::Vars(Vars::new(BTreeMap::new()))
    }
}

impl Render for ExtVars {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        match self {
            Self::Vars(vars) => Ok(Self::Vars(vars.render(context, place)?)),
            Self::Maps(maps) => Ok(Self::Maps(maps.render(context, place)?)),
        }
    }
}

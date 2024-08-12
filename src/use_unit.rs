use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::de::IntoDeserializer;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct UseUnit {
    pub module: PathBuf,
    pub prefix: Option<String>,
    pub items: BTreeSet<String>,
}

impl<'de> Deserialize<'de> for UseUnit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[serde(remote = "UseUnit")]
        struct UseUnitInner {
            module: PathBuf,
            prefix: Option<String>,
            #[serde(default)]
            items: BTreeSet<String>,
        }

        let value = serde_value::Value::deserialize(deserializer)?;
        if let Ok(module) = PathBuf::deserialize(value.clone().into_deserializer()) {
            Ok(Self { module, items: Default::default(), prefix: None })
        } else {
            UseUnitInner::deserialize(value.into_deserializer()).map_err(|e| e.to_error())
        }
    }
}

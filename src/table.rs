use std::collections::BTreeMap;

use anyhow::Result;
use cmd_lib::run_fun;
use serde::{Deserialize, Serialize};

use crate::render::Render;
use crate::string_or_int::StringOrInt;
use crate::template::Context;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TableFormat {
    Csv,
    Json,
    Toml,
    Yaml,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TableCommand {
    pub command: String,
    pub format: TableFormat,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Table {
    Maps(Vec<BTreeMap<String, StringOrInt>>),
    Command(TableCommand),
}

impl Default for Table {
    fn default() -> Self {
        Table::Maps(vec![])
    }
}

impl Table {
    pub fn list(&self, context: &Context) -> Result<Vec<BTreeMap<String, String>>> {
        let table: Vec<BTreeMap<String, StringOrInt>> = match self {
            Table::Maps(maps) => {
                let mut new_maps = Vec::with_capacity(maps.len());
                for map in maps {
                    let mut new_map = BTreeMap::new();
                    for (key, value) in map {
                        let new_value = value.render(context, "list table inline maps")?;
                        new_map.insert(key.to_string(), new_value);
                    }
                    new_maps.push(new_map);
                }
                new_maps
            }
            Table::Command(command) => {
                let cmd = command.command.render(context, "list table command")?;
                let out = run_fun!(sh -c $cmd)?;
                match command.format {
                    TableFormat::Toml => toml::from_str(&out)?,
                    TableFormat::Json => serde_json::from_str(&out)?,
                    TableFormat::Yaml => serde_yaml::from_str(&out)?,
                    TableFormat::Csv => {
                        let mut table = vec![];
                        let mut rdr = csv::Reader::from_reader(out.as_bytes());
                        for result in rdr.deserialize() {
                            let record: BTreeMap<String, StringOrInt> = result?;
                            table.push(record)
                        }
                        table
                    }
                }
            }
        };

        let result = table
            .into_iter()
            .map(|m| m.into_iter().map(|(k, v)| (k, v.to_string())).collect())
            .collect();

        Ok(result)
    }
}

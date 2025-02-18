use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use cmd_lib::run_fun;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::render::Render;
use crate::string_or_int::StringOrInt;
use crate::template::Context;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum StringOrUsize {
    String(String),
    Usize(usize),
}

impl StringOrUsize {
    pub fn to_usize(&self, field: &str) -> Result<usize> {
        match self {
            StringOrUsize::Usize(u) => Ok(*u),
            StringOrUsize::String(s) => {
                let u = s.parse::<usize>().with_context(|| format!("field: {field}"))?;
                Ok(u)
            }
        }
    }
}

impl Render for StringOrUsize {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        match self {
            StringOrUsize::Usize(_u) => Ok(self.to_owned()),
            StringOrUsize::String(s) => Ok(StringOrUsize::String(s.render(context, place)?)),
        }
    }
}

fn default_items_seq_start() -> StringOrUsize {
    StringOrUsize::Usize(0)
}

fn default_items_seq_step() -> StringOrUsize {
    StringOrUsize::Usize(1)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsSeq {
    #[serde(default = "default_items_seq_start")]
    start: StringOrUsize,
    end: StringOrUsize,
    #[serde(default = "default_items_seq_step")]
    step: StringOrUsize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsCommand {
    #[serde(alias = "cmd")]
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsVariable {
    #[serde(alias = "var")]
    pub variable: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsJson {
    pub json: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Items {
    Words(Vec<StringOrInt>),
    Seq(ItemsSeq),
    Command(ItemsCommand),
    Json(ItemsJson),
    Variable(ItemsVariable),
}

impl Items {
    pub fn list(&self, context: &Context) -> Result<Vec<String>> {
        let items = match self {
            Items::Words(words) => {
                words.iter().map(|w| w.to_string()).collect::<Vec<_>>().to_owned()
            }
            Items::Seq(seq) => {
                let start = seq.start.render(context, "list items start")?.to_usize("start")?;
                let end = seq.end.render(context, "list items end")?.to_usize("end")?;
                let step = seq.step.render(context, "list items step")?.to_usize("step")?;
                (start..end).step_by(step).map(|i| i.to_string()).collect::<Vec<String>>()
            }
            Items::Command(command) => {
                let cmd = command.command.render(context, "list items command")?;
                let out = run_fun!(sh -c $cmd)?;
                out.lines().map(|l| l.to_string()).collect::<Vec<String>>()
            }
            Items::Json(json) => {
                let json_str = json.json.render(context, "list items json")?;
                let json = serde_json::from_str(&json_str)?;
                match json {
                    Value::Array(a) => {
                        let mut items = Vec::with_capacity(a.len());
                        for item in a {
                            let item = match item {
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "".to_string(),
                                Value::Number(n) => n.to_string(),
                                Value::String(s) => s.to_string(),
                                _ => bail!(Error::WrongItemsJsonType(json_str)),
                            };
                            items.push(item);
                        }
                        items
                    }
                    Value::Object(o) => o.keys().map(|k| k.to_string()).collect(),
                    _ => bail!(Error::WrongItemsJsonType(json_str)),
                }
            }
            Items::Variable(variable) => {
                let var_name = variable.variable.render(context, "list items variable")?;
                let var = context
                    .get(&var_name)
                    .ok_or_else(|| Error::NoItemsVar(var_name.to_string()))?;
                match var {
                    Value::Array(a) => {
                        let mut items = Vec::with_capacity(a.len());
                        for item in a {
                            let item = match item {
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "".to_string(),
                                Value::Number(n) => n.to_string(),
                                Value::String(s) => s.to_string(),
                                _ => bail!(Error::WrongItemsVarType(var_name)),
                            };
                            items.push(item);
                        }
                        items
                    }
                    Value::Object(o) => o.keys().map(|k| k.to_string()).collect(),
                    _ => bail!(Error::WrongItemsVarType(var_name)),
                }
            }
        };

        Ok(items)
    }
}

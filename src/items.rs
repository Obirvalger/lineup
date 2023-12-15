use anyhow::Result;
use cmd_lib::run_fun;
use serde::{Deserialize, Serialize};

use crate::render::Render;
use crate::string_or_int::StringOrInt;
use crate::template::Context;

fn default_items_seq_start() -> usize {
    0
}

fn default_items_seq_step() -> usize {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsSeq {
    #[serde(default = "default_items_seq_start")]
    pub start: usize,
    pub end: usize,
    #[serde(default = "default_items_seq_step")]
    pub step: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ItemsCommand {
    #[serde(alias = "cmd")]
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Items {
    Words(Vec<StringOrInt>),
    Seq(ItemsSeq),
    Command(ItemsCommand),
}

impl Items {
    pub fn list(&self, context: &Context) -> Result<Vec<String>> {
        let items = match self {
            Items::Words(words) => {
                words.iter().map(|w| w.to_string()).collect::<Vec<_>>().to_owned()
            }
            Items::Seq(seq) => (seq.start..seq.end)
                .step_by(seq.step)
                .map(|i| i.to_string())
                .collect::<Vec<String>>(),
            Items::Command(command) => {
                let cmd = command.command.render(context, "list items command")?;
                let out = run_fun!(sh -c $cmd)?;
                out.lines().map(|l| l.to_string()).collect::<Vec<String>>()
            }
        };

        Ok(items)
    }
}

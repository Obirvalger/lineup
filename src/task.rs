use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use log::info;
use rayon_cond::CondIterator;
use serde::{Deserialize, Serialize};

use crate::items::Items;
use crate::manifest::Tasklines;
use crate::render::Render;
use crate::table::Table;
use crate::task_type::{CmdParams, TaskType};
use crate::template::Context;
use crate::vars::ExtVars;
use crate::worker::Worker;

fn default_task_items_table_items_var() -> String {
    "item".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskItemsTable {
    pub items: Items,
    #[serde(default = "default_task_items_table_items_var")]
    pub items_var: String,
    #[serde(rename = "table_by_item")]
    pub table_by_item: Option<Table>,
}

fn default_task_parallel() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Task {
    pub table: Option<Table>,
    #[serde(alias = "cond")]
    #[serde(alias = "if")]
    pub condition: Option<String>,
    #[serde(default)]
    pub clean_vars: bool,
    #[serde(default = "default_task_parallel")]
    pub parallel: bool,
    #[serde(default)]
    pub vars: ExtVars,
    #[serde(flatten)]
    pub items_table: Option<TaskItemsTable>,
    #[serde(flatten)]
    pub task_type: TaskType,
}

impl Task {
    pub fn run<S: AsRef<str>>(
        &self,
        name: &Option<S>,
        context: &Context,
        dir: &Path,
        tasklines: &Tasklines,
        worker: &Worker,
    ) -> Result<()> {
        let task_vars = self.vars.render(context, "task")?;
        let mut context = if self.clean_vars { Context::default() } else { context.to_owned() };
        context.extend(task_vars.vars()?.context()?);

        let items = self
            .items_table
            .as_ref()
            .map(|i| i.items.list(&context))
            .transpose()?
            .unwrap_or_else(|| vec!["".to_string()]);
        let items_var = self
            .items_table
            .as_ref()
            .map(|t| t.items_var.to_string())
            .unwrap_or_else(|| "item".to_string());

        let name = name.as_ref().map(|n| n.as_ref().to_string());
        CondIterator::new(items, self.parallel)
            .map(|item| -> Result<()> {
                let table = self
                    .table
                    .as_ref()
                    .map(|i| i.list(&context))
                    .transpose()?
                    .unwrap_or_else(|| vec![BTreeMap::new()]);
                let mut context = context.to_owned();
                context.insert(&items_var, &item);
                if let Some(items_table) = &self.items_table {
                    if let Some(table_by_item) = &items_table.table_by_item {
                        for row in table_by_item.list(&context)? {
                            if let Some(table_item) = row.get("item") {
                                if table_item == &item {
                                    context.insert("row_by_item", &row);
                                }
                            }
                        }
                    }
                }

                CondIterator::new(table, self.parallel)
                    .map(|row| -> Result<()> {
                        let mut context = context.to_owned();
                        context.insert("row", &row);
                        if let Some(condition) = &self.condition {
                            let condition = condition.render(&context, "task condition")?;
                            if worker.shell(condition, &CmdParams::default()).is_err() {
                                return Ok(());
                            }
                        }
                        if let Some(name) = &name {
                            let name = name.render(&context, "task name")?;
                            if self.items_table.is_some() {
                                info!(
                                    "Run task `{}` (item={}) on worker `{}`",
                                    name, &item, &worker.name
                                );
                            } else if self.table.is_some() {
                                info!(
                                    "Run task `{}` (row={}) on worker `{}`",
                                    name,
                                    serde_json::to_string(&row)?,
                                    &worker.name
                                );
                            } else {
                                info!("Run task `{}` on worker `{}`", name, &worker.name);
                            };
                        }
                        self.task_type.run(&context, dir, tasklines, worker)
                    })
                    .find_any(|r| r.is_err())
                    .unwrap_or(Ok(()))
            })
            .find_any(|r| r.is_err())
            .unwrap_or(Ok(()))
    }
}

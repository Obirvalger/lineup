use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context as AnyhowContext;
use anyhow::Result;
use log::{info, warn};
use rayon::iter::ParallelIterator;
use rayon_cond::CondIterator;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fs_var::FsVar;
use crate::items::Items;
use crate::manifest::Tasklines;
use crate::render::Render;
use crate::storage::Storages;
use crate::table::Table;
use crate::task_result::TaskResult;
use crate::task_type::{CmdParams, TaskType};
use crate::template::Context;
use crate::vars::ExtVars;
use crate::worker::Worker;

#[derive(Clone, Debug)]
pub struct Env<'a> {
    pub dir: &'a PathBuf,
    pub storages: &'a Storages,
    pub tasklines: &'a Tasklines,
    pub workers: &'a Vec<Worker>,
}

fn show_duration(duration: Duration) -> String {
    let ms = duration.as_millis();
    if ms < 2000 {
        format!("{} ms", ms)
    } else {
        format!("{} s", duration.as_secs())
    }
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TaskTryCleanup {
    task: TaskType,
}

fn default_task_try_sleep() -> Duration {
    Duration::from_secs(1)
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TaskTry {
    attempts: NonZeroU32,
    cleanup: Option<TaskTryCleanup>,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64, serde_with::formats::Flexible>")]
    #[serde(default = "default_task_try_sleep")]
    sleep: Duration,
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
    pub result_fs_var: Option<String>,
    #[serde(default)]
    pub vars: ExtVars,
    #[serde(default)]
    pub export_vars: Vec<String>,
    #[serde(flatten)]
    pub items_table: Option<TaskItemsTable>,
    #[serde(flatten)]
    pub task_type: TaskType,
    #[serde(rename = "try")]
    pub try_: Option<TaskTry>,
}

impl Task {
    pub fn run<S: AsRef<str>>(
        &self,
        name: &Option<S>,
        context: &Context,
        env: &Env,
        worker: &Worker,
    ) -> Result<TaskResult> {
        let context = if self.clean_vars { Context::default() } else { context.to_owned() };

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
        let results =
            CondIterator::new(items, self.parallel).map(|item| -> Result<(String, TaskResult)> {
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

                let results = CondIterator::new(table, self.parallel)
                    .map(|row| -> Result<TaskResult> {
                        let mut context = context.to_owned();
                        context.insert("row", &row);
                        let task_vars = self.vars.render(&context, "task")?;
                        let vars_context = task_vars.vars()?.context()?;
                        context.extend(vars_context.to_owned());
                        if let Some(condition) = &self.condition {
                            let condition = condition.render(&context, "task condition")?;
                            let skip = match condition.trim() {
                                "true" => false,
                                "false" => true,
                                _ => worker.shell(condition, &CmdParams::default()).is_err(),
                            };
                            if skip {
                                let result = context.get("result").unwrap_or(&Value::Null);
                                return Ok(result.to_owned().into());
                            }
                        }
                        if let Some(name) = &name {
                            let name = name.render(&context, "task name")?;
                            if self.items_table.is_some() {
                                info!(
                                    "Run task `{}` (item={}) on worker `{}`",
                                    name,
                                    &item,
                                    worker.name()
                                );
                            } else if self.table.is_some() {
                                info!(
                                    "Run task `{}` (row={}) on worker `{}`",
                                    name,
                                    serde_json::to_string(&row)?,
                                    worker.name()
                                );
                            } else {
                                info!("Run task `{}` on worker `{}`", name, worker.name());
                            };
                        }

                        let start = Instant::now();
                        let mut attempts = "".to_string();
                        let mut res = self.task_type.run(&context, env, worker);
                        if let Some(try_) = &self.try_ {
                            let mut final_attempt = 1;
                            for attempt in 1..=try_.attempts.get() {
                                final_attempt = attempt;
                                if res.is_err() {
                                    thread::sleep(try_.sleep);
                                    if let Some(cleanup) = &try_.cleanup {
                                        if cleanup.task.run(&context, env, worker).is_err() {
                                            warn!("Cleanup command failed");
                                        }
                                    }
                                    res = self.task_type.run(&context, env, worker);
                                } else {
                                    break;
                                }
                            }

                            if final_attempt > 1 {
                                attempts = format!("({} attempts)", final_attempt);
                            }
                        }
                        let duration = start.elapsed();

                        if let Some(name) = &name {
                            let name = name.render(&context, "task name")?;
                            if self.items_table.is_some() {
                                info!(
                                    "Task `{}` (item={}) on worker `{}` finished{} in {}",
                                    name,
                                    &item,
                                    worker.name(),
                                    attempts,
                                    show_duration(duration),
                                );
                            } else if self.table.is_some() {
                                info!(
                                    "Task `{}` (row={}) on worker `{}` finished{} in {}",
                                    name,
                                    serde_json::to_string(&row)?,
                                    worker.name(),
                                    attempts,
                                    show_duration(duration),
                                );
                            } else {
                                info!(
                                    "Task `{}` on worker `{}` finished{} in {}",
                                    name,
                                    worker.name(),
                                    attempts,
                                    show_duration(duration),
                                );
                            };
                        }

                        if self.items_table.is_some() {
                            res = res.with_context(|| format!("item: `{}`", item));
                        }
                        let mut new_vars_context = Context::new();
                        for name in &self.export_vars {
                            if let Some(value) = &vars_context.get(name) {
                                new_vars_context.insert(name.to_string(), value);
                            }
                        }
                        let _ = res.as_mut().map(|r| r.add_vars(new_vars_context.into_json()));
                        res
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let result = if self.table.is_some() {
                    TaskResult::fold_vec(&results)
                } else {
                    results[0].to_owned()
                };

                Ok((item, result))
            });

        let result = match results {
            CondIterator::Serial(mut iterator) => {
                if self.items_table.is_some() {
                    let mut pairs = vec![];
                    for result in iterator {
                        let (item, result) = result?;
                        if result.as_exception().is_some() {
                            return Ok(result);
                        }
                        pairs.push((item, result));
                    }
                    TaskResult::fold_items(&pairs)
                } else {
                    iterator.next().expect("No one result of task without items")?.1
                }
            }
            CondIterator::Parallel(iterator) => {
                let results = iterator.collect::<Result<Vec<_>>>()?;
                if self.items_table.is_some() {
                    TaskResult::fold_items(&results)
                } else {
                    results[0].1.to_owned()
                }
            }
        };

        if let Some(fs_var_name) = &self.result_fs_var {
            if let Some(value) = result.as_value() {
                let fs_var_name = fs_var_name.render(&context, "task result-fs-var")?;
                let fs_var = FsVar::new(fs_var_name)?;
                fs_var.write(value)?;
            }
        }

        Ok(result)
    }
}

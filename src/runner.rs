use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use anyhow::Result;
use rayon::prelude::*;
use regex::RegexSet;

use crate::engine::ExistsAction;
use crate::error::Error;
use crate::manifest::{Manifest, Tasklines, Taskset};
use crate::render::Render;
use crate::template::Context;
use crate::tsort::tsort;
use crate::vars::Vars;
use crate::worker::Worker;

#[derive(Clone, Debug)]
pub struct Runner {
    pub taskset: Taskset,
    pub skip_tasks: Vec<String>,
    pub tasklines: Tasklines,
    pub vars: Vars,
    pub workers: Vec<Worker>,
    pub dir: PathBuf,
    worker_exists: Option<ExistsAction>,
}

impl Runner {
    pub fn from_manifest<S: AsRef<OsStr>>(manifest_path: S) -> Result<Self> {
        let manifest_path = Path::new(manifest_path.as_ref());
        let dir = manifest_path
            .parent()
            .ok_or_else(|| Error::BadManifest(manifest_path.to_owned()))?
            .to_owned();
        let manifest_str = fs::read_to_string(manifest_path)
            .with_context(|| format!("Failed to read manifest `{}`", &manifest_path.display()))?;
        let manifest: Manifest = toml::from_str(&manifest_str)
            .with_context(|| format!("Failed to parse manifest `{}`", &manifest_path.display()))?;
        let defaults = &manifest.default;
        let vars = manifest.vars.to_owned();
        let taskset = manifest.taskset.to_owned();
        let mut tasklines = manifest.tasklines.to_owned();
        if !manifest.taskline.is_empty() {
            tasklines.insert("".to_string(), manifest.taskline.to_owned());
        }
        let context = manifest.vars.context()?;
        let workers =
            Worker::from_manifest_workers(&manifest.workers, &defaults.worker, &context)?;
        let worker_exists = None;
        let skip_tasks = vec![];
        Ok(Self { dir, taskset, skip_tasks, tasklines, vars, workers, worker_exists })
    }

    pub fn add_extra_vars(&mut self, vars: Vars) {
        self.vars.extend(vars);
    }

    pub fn render_vars(&mut self, context: &Context) -> Result<()> {
        self.vars = self.vars.render(context, "manifest")?;

        Ok(())
    }

    pub fn skip_tasks(&mut self, tasks: &[String]) {
        self.skip_tasks = Vec::from(tasks);
    }

    pub fn set_worker_exists_action(&mut self, action: Option<ExistsAction>) {
        self.worker_exists = action;
    }

    pub fn clean(&mut self) -> Result<()> {
        for worker in &mut self.workers {
            worker.ensure_remove()?;
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let tasks_graph = self
            .taskset
            .iter()
            .map(|(n, t)| (n.to_string(), t.requires.to_owned()))
            .collect::<BTreeMap<_, _>>();

        for layer in tsort(&tasks_graph, "taskset requires")? {
            let mut workers_by_task = BTreeMap::new();

            // setup workers by task sequentially to ensure the same worker does not run
            // setup in parallel
            for name in &layer {
                let taskset_elem =
                    self.taskset.get(name).ok_or(Error::BadTaskInTaskset(name.to_string()))?;
                let workers_re_set = RegexSet::new(&taskset_elem.workers)?;
                let worker_names = self
                    .workers
                    .par_iter_mut()
                    .filter_map(|worker| -> Option<Result<String>> {
                        if workers_re_set.is_match(&worker.name) {
                            if let Err(error) = worker.ensure_setup(&self.worker_exists) {
                                return Some(Err(error));
                            }
                            Some(Ok(worker.name.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                workers_by_task.insert(name, worker_names);
            }

            layer.par_iter().try_for_each(|name| -> Result<()> {
                if self.skip_tasks.contains(name) {
                    return Ok(());
                }

                let taskset_elem =
                    self.taskset.get(name).ok_or(Error::BadTaskInTaskset(name.to_string()))?;
                let task = &taskset_elem.task;
                self.workers.par_iter().try_for_each(|worker| -> Result<()> {
                    let mut context = self.vars.context()?;
                    context.insert("worker", &worker.name);
                    if workers_by_task
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                        .contains(&worker.name)
                    {
                        task.run(&Some(name), &context, &self.dir, &self.tasklines, worker)?;
                    };

                    Ok(())
                })
            })?;
        }

        Ok(())
    }
}

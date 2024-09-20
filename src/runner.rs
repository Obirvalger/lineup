use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use rayon::prelude::*;
use regex::RegexSet;

use crate::engine::ExistsAction;
use crate::error::Error;
use crate::manifest::{Manifest, Tasklines, Taskset};
use crate::module;
use crate::render::Render;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::tsort::tsort;
use crate::use_unit::UseUnit;
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
    fn get_use_tasklines(
        context: &Context,
        dir: &Path,
        use_units: &[UseUnit],
    ) -> Result<Tasklines> {
        let mut tasklines = BTreeMap::new();

        for use_unit in use_units {
            let module = module::resolve(&use_unit.module, dir);
            let manifest = Self::from_manifest(&module, context)?;
            let mut use_tasklines = manifest.tasklines;

            if !use_unit.items.is_empty() {
                use_tasklines.retain(|k, _| use_unit.items.contains(k));
                let taskline_names = use_tasklines.keys().cloned().collect::<BTreeSet<_>>();
                let diff = use_unit.items.difference(&taskline_names).collect::<BTreeSet<_>>();
                if !diff.is_empty() {
                    let tasklines_s = diff.into_iter().cloned().collect::<Vec<_>>().join(", ");
                    bail!(Error::UseTasklines(tasklines_s, module))
                }
            }

            let prefix = use_unit.prefix.to_owned().unwrap_or_else(|| {
                module.file_stem().expect("empty module filename").to_string_lossy().to_string()
            });
            if !prefix.is_empty() {
                use_tasklines = use_tasklines
                    .into_keys()
                    .map(|name| {
                        (
                            if name.is_empty() {
                                prefix.to_string()
                            } else {
                                format!("{}.{}", prefix, name)
                            },
                            Taskline::File { file: module.to_owned(), name },
                        )
                    })
                    .collect();
            }

            tasklines.extend(use_tasklines);
        }

        Ok(tasklines)
    }

    fn get_use_vars(context: &Context, dir: &Path, use_units: &[UseUnit]) -> Result<Vars> {
        let mut vars = Vars::new();

        for use_unit in use_units {
            let module = module::resolve(&use_unit.module, dir);
            let mut use_vars = Self::from_manifest(&module, context)?.vars.into_map();

            if !use_unit.items.is_empty() {
                use_vars.retain(|k, _| use_unit.items.contains(k));
                let var_names = use_vars.keys().cloned().collect::<BTreeSet<_>>();
                let diff = use_unit.items.difference(&var_names).collect::<BTreeSet<_>>();
                if !diff.is_empty() {
                    let vars_s = diff.into_iter().cloned().collect::<Vec<_>>().join(", ");
                    bail!(Error::UseVars(vars_s, module))
                }
            }

            if let Some(prefix) = &use_unit.prefix {
                if !prefix.is_empty() {
                    use_vars = BTreeMap::from([(prefix.to_string(), serde_json::json!(use_vars))]);
                }
            } else {
                let prefix = module
                    .file_stem()
                    .expect("empty module filename")
                    .to_string_lossy()
                    .to_string()
                    .replace('-', "_");
                use_vars = BTreeMap::from([(prefix, serde_json::json!(use_vars))]);
            }

            vars.extend(Vars::try_from(use_vars)?);
        }

        Ok(vars)
    }

    pub fn from_manifest<S: AsRef<OsStr>>(manifest_path: S, context: &Context) -> Result<Self> {
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

        let place = "Runner::from_manifest";
        let mut context = context.to_owned();
        let mut vars = Self::get_use_vars(&context, &dir, &manifest.use_.vars)?;
        let mut new_context = vars.context()?;
        new_context.extend(context);
        context = new_context;
        vars.extend(manifest.vars.to_owned().render(&context, place)?);
        new_context = vars.context()?;
        new_context.extend(context);
        context = new_context;
        let maps_vars = manifest.extend.vars.maps.render(&context, place)?.vars()?;
        vars.extend(maps_vars);
        context.extend(vars.context()?);

        let taskset = manifest.taskset.to_owned();

        let mut tasklines = Self::get_use_tasklines(&context, &dir, &manifest.use_.tasklines)?;
        let mut manifest_tasklines = manifest.tasklines.to_owned();
        if !manifest.taskline.is_empty() {
            manifest_tasklines
                .insert("".to_string(), Taskline::Line(manifest.taskline.to_owned()));
        }
        tasklines.extend(manifest_tasklines);

        let workers =
            Worker::from_manifest_workers(&manifest.workers, &defaults.worker, &context)?;
        let worker_exists = None;
        let skip_tasks = vec![];

        Ok(Self { dir, taskset, skip_tasks, tasklines, vars, workers, worker_exists })
    }

    pub fn add_extra_vars(&mut self, vars: Vars) {
        self.vars.extend(vars);
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

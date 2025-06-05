use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use log::warn;
use rayon::prelude::*;
use regex::RegexSet;
use serde_json::Value;

use crate::engine::ExistsAction;
use crate::error::Error;
use crate::manifest::{Manifest, Tasklines, Taskset};
use crate::module;
use crate::network::Network;
use crate::render::Render;
use crate::storage::{Storage, Storages};
use crate::task::Env;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::tsort::tsort;
use crate::use_unit::UseUnit;
use crate::vars::Vars;
use crate::worker::Worker;

fn save_layers(layers: &Vec<Vec<String>>) -> Result<()> {
    if let Ok(layers_file) = env::var("LINEUP_LAYERS") {
        let context = format!("save layers to `{}`", layers_file);
        fs::write(
            layers_file,
            serde_json::to_string_pretty(layers).with_context(|| context.to_string())?,
        )
        .context(context)?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct Runner {
    pub taskset: Taskset,
    pub skip_tasks: Vec<String>,
    pub tasklines: Tasklines,
    pub vars: Vars,
    pub networks: Vec<Network>,
    pub storages: Storages,
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
            .canonicalize()
            .with_context(|| format!("Failed to find manifest `{}`", &manifest_path.display()))?
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
        context.insert("manifest_dir", &dir.to_string_lossy().to_string());
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

        let networks = Network::from_manifest_networks(&manifest.networks, &context)?;
        let storages = Storage::from_manifest_storages(&manifest.storages, &context)?;

        let workers =
            Worker::from_manifest_workers(&manifest.workers, &defaults.worker, &context, &dir)?;
        let worker_exists = None;
        let skip_tasks = vec![];

        Ok(Self {
            dir,
            taskset,
            skip_tasks,
            tasklines,
            vars,
            networks,
            storages,
            workers,
            worker_exists,
        })
    }

    pub fn add_extra_vars(&mut self, vars: Vars) {
        self.vars.extend(vars);
    }

    pub fn set_storages(&mut self, storages: &Storages) {
        self.storages = storages.to_owned();
    }

    pub fn set_workers(&mut self, workers: &[Worker]) {
        self.workers = Vec::from(workers);
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

        for network in &mut self.networks {
            network.remove()?;
        }

        for storage in self.storages.values_mut() {
            storage.remove()?;
        }

        Ok(())
    }

    fn setup_networks(&self) -> Result<()> {
        for network in &self.networks {
            network.setup()?;
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        if self.workers.is_empty() {
            bail!(Error::NoWorkers)
        }
        let mut context = Context::new();
        context.insert("result", &Value::Null);
        context.extend(self.vars.context()?);
        context.insert("manifest_dir", &self.dir.to_string_lossy().to_string());

        let tasks_graph = self
            .taskset
            .iter()
            .map(|(n, t)| (n.to_string(), t.requires.to_owned()))
            .collect::<BTreeMap<_, _>>();

        self.setup_networks()?;

        let layers = tsort(&tasks_graph, "taskset requires")?;
        save_layers(&layers)?;

        for layer in layers {
            let mut workers_by_task = BTreeMap::new();

            // setup workers by task sequentially to ensure the same worker does not run
            // setup in parallel
            for name in &layer {
                let taskset_elem =
                    self.taskset.get(name).ok_or(Error::BadTaskInTaskset(name.to_string()))?;
                let workers_re =
                    taskset_elem.workers.iter().map(|w| format!("^{w}$")).collect::<Vec<_>>();
                let workers_re_set = RegexSet::new(&workers_re)?;
                let worker_names = self
                    .workers
                    .par_iter_mut()
                    .filter_map(|worker| -> Option<Result<String>> {
                        if workers_re_set.is_match(&worker.name()) {
                            if let Err(error) =
                                worker.ensure_setup(&self.worker_exists, &self.storages)
                            {
                                return Some(Err(error));
                            }
                            Some(Ok(worker.name()))
                        } else {
                            None
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                if worker_names.is_empty() {
                    bail!(Error::NoWorkersForTask(name.to_string()));
                } else {
                    workers_by_task.insert(name, worker_names);
                }
            }

            layer.par_iter().try_for_each(|name| -> Result<()> {
                if self.skip_tasks.contains(name) {
                    return Ok(());
                }

                let taskset_elem =
                    self.taskset.get(name).ok_or(Error::BadTaskInTaskset(name.to_string()))?;
                let provide_workers = self
                    .workers
                    .iter()
                    .filter(|w| taskset_elem.provide_workers.contains(&w.name()))
                    .map(|w| w.to_owned())
                    .collect::<Vec<_>>();
                let task = &taskset_elem.task;

                let env = Env {
                    dir: &self.dir,
                    storages: &self.storages,
                    tasklines: &self.tasklines,
                    workers: &provide_workers,
                };

                self.workers.par_iter().try_for_each(|worker| -> Result<()> {
                    if workers_by_task
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                        .contains(&worker.name())
                    {
                        let mut context = context.to_owned();
                        context.insert("worker", &worker.name());
                        let result =
                            task.run(&Some(name), &context, &env, worker).with_context(|| {
                                format!("taskset task: `{}`, worker: `{}`", name, worker.name())
                            })?;
                        if let Some(exception) = result.as_exception() {
                            warn!("Got exception: {:?}", exception);
                        }
                    };

                    Ok(())
                })
            })?;
        }

        Ok(())
    }
}

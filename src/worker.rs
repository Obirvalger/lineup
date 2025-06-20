use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::cmd::CmdOut;
use crate::engine::{Engine, ExistsAction};
use crate::error::Error;
use crate::manifest::DefaultWorker;
use crate::manifest::Workers as ManifestWorkers;
use crate::render::Render;
use crate::storage::Storages;
use crate::task_type::{CmdParams, SpecialTypeType};
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct Worker {
    name: String,
    name_outer: String,
    workdir: PathBuf,
    engine: Engine,
    setup: bool,
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Worker {}

impl PartialOrd for Worker {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Worker {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Worker {
    pub fn from_manifest_workers(
        manifest_workers: &ManifestWorkers,
        default: &DefaultWorker,
        context: &Context,
        dir: &Path,
    ) -> Result<Vec<Self>> {
        let mut workers = BTreeSet::new();
        let mut context = context.to_owned();
        for (name, worker) in manifest_workers {
            let items = worker
                .items
                .as_ref()
                .or(default.items.as_ref())
                .map(|i| i.list(&context))
                .transpose()?
                .unwrap_or_else(|| vec!["".to_string()]);
            for item in items {
                context.insert("item", &item);
                for row in &worker.table_by_item.list(&context)? {
                    if let Some(table_item) = row.get("item") {
                        if *table_item == item {
                            context.insert("row_by_item", &row);
                        }
                    }
                }

                let name = name.render(&context, "name in workers in manifest")?;
                let name_outer = name.to_string();
                for row in &worker.table_by_name.list(&context)? {
                    if let Some(table_name) = row.get("name") {
                        let table_name = table_name
                            .render(&context, "name in table_by_name in workers in manifest")?;
                        if *table_name == name {
                            context.insert("row_by_name", &row);
                        }
                    }
                }

                let engine = worker
                    .engine
                    .as_ref()
                    .or(default.engine.as_ref())
                    .ok_or_else(|| Error::NoEngine(name.to_string()))?;
                let engine = Engine::from_manifest_engine(&context, engine, dir)?;
                workers.insert(Worker {
                    name,
                    name_outer,
                    engine,
                    setup: false,
                    workdir: PathBuf::default(),
                });
            }
        }

        Ok(workers.into_iter().collect::<Vec<Self>>())
    }

    pub fn name(&self) -> String {
        self.name_outer.to_string()
    }

    pub fn rename<S: AsRef<str>>(&mut self, new_name: S) -> String {
        let old_name = self.name_outer.to_string();
        self.name_outer = new_name.as_ref().to_string();

        old_name
    }

    pub fn ensure_setup(
        &mut self,
        action: &Option<ExistsAction>,
        storages: &Storages,
    ) -> Result<()> {
        if !self.setup {
            self.engine.setup(&self.name, action, storages)?;
            let cmd = "echo ${TMPDIR:-${TMP:-/tmp}}/lineup";
            let out = self.engine.shell_out(&self.name, cmd, &None)?;
            if !out.success() {
                bail!(Error::WorkerSetupFailed(self.name.to_string()))
            }
            self.workdir = PathBuf::from(out.stdout());
            self.setup = true;
        }

        Ok(())
    }

    pub fn ensure_remove(&mut self) -> Result<()> {
        self.engine.remove(&self.name)?;
        self.setup = false;

        Ok(())
    }

    pub fn copy<S: AsRef<Path>, D: AsRef<Path>>(&self, src: S, dst: D) -> Result<()> {
        self.engine.copy(&self.name, src, dst)?;

        Ok(())
    }

    pub fn get<S: AsRef<Path>, D: AsRef<Path>>(&self, src: S, dst: D) -> Result<()> {
        self.engine.get(&self.name, src, dst)?;

        Ok(())
    }

    pub fn exec<S: AsRef<str>>(&self, args: &[S], params: &CmdParams) -> Result<CmdOut> {
        self.engine.exec(&self.name, args, params)
    }

    pub fn shell<S: AsRef<str>>(&self, command: S, params: &CmdParams) -> Result<CmdOut> {
        self.engine.shell(&self.name, command, params)
    }

    pub fn special(&self, type_: &SpecialTypeType, ignore_unsupported: bool) -> Result<()> {
        self.engine.special(&self.name, type_, ignore_unsupported)
    }
}

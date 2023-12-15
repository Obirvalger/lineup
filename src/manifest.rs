use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::engine::{EngineBase, ExistsAction};
use crate::items::Items;
use crate::render::Render;
use crate::string_or_int::StringOrInt;
use crate::table::Table;
use crate::task::Task;
use crate::task_type::TaskType;
use crate::template::Context;
use crate::vars::Vars;

pub type Workers = BTreeMap<String, Worker>;
pub type Taskline = Vec<TasklineElem>;
pub type Tasklines = BTreeMap<String, Taskline>;
pub type Taskset = BTreeMap<String, TasksetElem>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultWorker {
    pub items: Option<Items>,
    pub table_by_item: Option<Table>,
    pub table_by_name: Option<Table>,
    pub engine: Option<Engine>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    #[serde(default)]
    pub worker: DefaultWorker,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineVmlNetTap {
    pub tap: String,
    pub address: Option<String>,
    pub gateway: Option<String>,
    pub nameservers: Option<Vec<String>>,
}

impl Render for EngineVmlNetTap {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("net tap in {}", place.as_ref());
        let tap = self.tap.render(context, format!("tap in {}", place))?;
        let address = self.address.render(context, format!("address in {}", place))?;
        let gateway = self.gateway.render(context, format!("gateway in {}", place))?;
        let nameservers = self.nameservers.render(context, format!("nameservers in {}", place))?;
        Ok(Self { tap, address, gateway, nameservers })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum EngineVmlNet {
    User,
    #[serde(untagged)]
    Tap(EngineVmlNetTap),
}

impl Render for EngineVmlNet {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        match self {
            EngineVmlNet::User => Ok(self.to_owned()),
            EngineVmlNet::Tap(engine_vml_net_tap) => {
                Ok(EngineVmlNet::Tap(engine_vml_net_tap.render(context, place)?))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineVml {
    #[serde(alias = "vml_bin")]
    pub vml_bin: Option<String>,
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub image: Option<String>,
    pub net: Option<EngineVmlNet>,
    pub nproc: Option<StringOrInt>,
    pub parent: Option<String>,
    pub user: Option<String>,
    #[serde(default)]
    pub exists: ExistsAction,
    #[serde(flatten)]
    #[serde(default)]
    pub base: EngineBase,
}

impl Render for EngineVml {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("vml engine in {}", place.as_ref());
        let memory = self.memory.render(context, format!("memory in {}", place))?;
        let image = self.image.render(context, format!("image in {}", place))?;
        let net = self.net.render(context, &place)?;
        let nproc = self.nproc.render(context, format!("nproc in {}", place))?;
        let parent = self.parent.render(context, format!("parent in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, image, net, nproc, parent, user, base, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineDocker {
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub image: String,
    pub user: Option<String>,
    #[serde(default)]
    pub exists: ExistsAction,
    #[serde(flatten)]
    #[serde(default)]
    pub base: EngineBase,
}

impl Render for EngineDocker {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("docker engine in {}", place.as_ref());
        let memory = self.memory.render(context, format!("memory in {}", place))?;
        let image = self.image.render(context, format!("image in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, image, user, base, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EnginePodman {
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub image: String,
    pub pod: Option<String>,
    pub user: Option<String>,
    #[serde(default)]
    pub exists: ExistsAction,
    #[serde(flatten)]
    #[serde(default)]
    pub base: EngineBase,
}

impl Render for EnginePodman {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("podman engine in {}", place.as_ref());
        let memory = self.memory.render(context, format!("memory in {}", place))?;
        let image = self.image.render(context, format!("image in {}", place))?;
        let pod = self.pod.render(context, format!("pod in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, image, pod, user, base, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Vml(EngineVml),
    Docker(EngineDocker),
    Podman(EnginePodman),
    Host,
}

fn default_worker_setup_engine() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Worker {
    pub items: Option<Items>,
    #[serde(default)]
    pub table_by_item: Table,
    #[serde(default)]
    pub table_by_name: Table,
    pub engine_name: Option<String>,
    pub engine: Option<Engine>,
    #[serde(default = "default_worker_setup_engine")]
    pub setup_engine: bool,
}

fn default_taskset_elem_workers() -> Vec<String> {
    vec![".*".to_string()]
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TasksetElem {
    #[serde(default)]
    pub requires: BTreeSet<String>,
    #[serde(default = "default_taskset_elem_workers")]
    pub workers: Vec<String>,
    #[serde(flatten)]
    pub task: Task,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TasklineElem {
    pub name: Option<String>,
    #[serde(flatten)]
    pub task: Task,
}

fn default_taskset() -> Taskset {
    let task_type = TaskType::RunTaskline(Default::default());
    let task = Task {
        table: None,
        condition: None,
        items_table: None,
        clean_vars: Default::default(),
        parallel: true,
        vars: Default::default(),
        task_type,
    };
    let taskset_elem = TasksetElem {
        requires: Default::default(),
        workers: default_taskset_elem_workers(),
        task,
    };
    BTreeMap::from([("Run taskline".to_owned(), taskset_elem)])
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    #[serde(default)]
    pub vars: Vars,
    #[serde(default)]
    pub default: Defaults,
    #[serde(default)]
    pub workers: Workers,
    #[serde(default = "default_taskset")]
    pub taskset: Taskset,
    #[serde(default)]
    pub taskline: Taskline,
    #[serde(default)]
    pub tasklines: Tasklines,
}

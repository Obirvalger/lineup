use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use crate::engine::{EngineBase, ExistsAction};
use crate::items::Items;
use crate::render::Render;
use crate::string_or_int::StringOrInt;
use crate::table::Table;
use crate::task::Task;
use crate::task_type::TaskType;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::use_unit::UseUnit;
use crate::vars::{Maps, Vars};

pub type Networks = BTreeMap<String, Network>;
pub type Storages = BTreeMap<String, Storage>;
pub type Workers = BTreeMap<String, Worker>;
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Use {
    #[serde(default)]
    pub vars: Vec<UseUnit>,
    #[serde(default)]
    pub tasklines: Vec<UseUnit>,
}

fn default_network_engine_incus_nat() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct NetworkEngineIncus {
    pub address: String,
    #[serde(default = "default_network_engine_incus_nat")]
    pub nat: bool,
}

impl Render for NetworkEngineIncus {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("incus network engine in {}", place.as_ref());
        let address = self.address.render(context, format!("address in {}", place))?;
        Ok(Self { address, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkEngine {
    Incus(NetworkEngineIncus),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub engine: NetworkEngine,
}

fn default_storage_engine_pool() -> String {
    "default".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct StorageEngineIncus {
    #[serde(default = "default_storage_engine_pool")]
    pub pool: String,
    pub copy: Option<String>,
}

impl Render for StorageEngineIncus {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("incus storage engine in {}", place.as_ref());
        let pool = self.pool.render(context, format!("pool in {}", place))?;
        let copy = self.copy.render(context, format!("copy in {}", place))?;
        Ok(Self { pool, copy })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageEngine {
    Incus(StorageEngineIncus),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub items: Option<Items>,
    pub engine: StorageEngine,
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

fn default_engine_ssh_ssh_cmd() -> Vec<String> {
    vec!["ssh".to_string()]
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineSsh {
    pub host: String,
    pub port: Option<String>,
    pub user: Option<String>,
    pub key: Option<String>,
    #[serde(alias = "ssh_cmd")]
    #[serde(default = "default_engine_ssh_ssh_cmd")]
    pub ssh_cmd: Vec<String>,
}

impl Render for EngineSsh {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("ssh engine in {}", place.as_ref());
        let host = self.host.render(context, format!("host in {}", place))?;
        let port = self.port.render(context, format!("port in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let key = self.key.render(context, format!("key in {}", place))?;
        Ok(Self { host, port, user, key, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineDocker {
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub image: String,
    pub load: Option<PathBuf>,
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
        let load = self.load.render(context, format!("load in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, image, load, user, base, ..self.to_owned() })
    }
}

fn default_engine_incus_net_device() -> String {
    "eth0".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineIncusNet {
    pub address: Option<String>,
    #[serde(default = "default_engine_incus_net_device")]
    pub device: String,
    pub network: Option<String>,
}

impl Render for EngineIncusNet {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let address = self.address.render(context, format!("address in {}", place.as_ref()))?;
        let device = self.device.render(context, format!("device in {}", place.as_ref()))?;
        let network = self.network.render(context, format!("network in {}", place.as_ref()))?;
        Ok(Self { address, device, network })
    }
}

fn default_engine_incus_storage_readonly() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineIncusStorage {
    pub path: PathBuf,
    #[serde(default = "default_storage_engine_pool")]
    pub pool: String,
    #[serde(default = "default_engine_incus_storage_readonly")]
    pub readonly: bool,
    pub volume: Option<String>,
}

impl Render for EngineIncusStorage {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let pool = self.pool.render(context, format!("pool in {}", place.as_ref()))?;
        let path = self.path.render(context, format!("path in {}", place.as_ref()))?;
        let volume = self.volume.render(context, format!("volume in {}", place.as_ref()))?;
        Ok(Self { pool, path, readonly: self.readonly, volume })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EngineIncus {
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub net: Option<EngineIncusNet>,
    pub nproc: Option<StringOrInt>,
    pub image: String,
    pub copy: Option<String>,
    #[serde(default)]
    pub storages: BTreeMap<String, EngineIncusStorage>,
    pub user: Option<String>,
    #[serde(default)]
    pub exists: ExistsAction,
    #[serde(flatten)]
    #[serde(default)]
    pub base: EngineBase,
}

impl Render for EngineIncus {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let place = format!("incus engine in {}", place.as_ref());
        let memory = self.memory.render(context, format!("memory in {}", place))?;
        let net = self.net.render(context, format!("net in {}", place))?;
        let nproc = self.nproc.render(context, format!("nproc in {}", place))?;
        let image = self.image.render(context, format!("image in {}", place))?;
        let copy = self.copy.render(context, format!("copy in {}", place))?;
        let storages = self.storages.render(context, format!("storages in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let exists = self.exists.to_owned();
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, net, nproc, image, copy, storages, user, exists, base })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct EnginePodman {
    #[serde(alias = "mem")]
    pub memory: Option<String>,
    pub image: String,
    pub load: Option<PathBuf>,
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
        let load = self.load.render(context, format!("load in {}", place))?;
        let pod = self.pod.render(context, format!("pod in {}", place))?;
        let user = self.user.render(context, format!("user in {}", place))?;
        let base = self.base.render(context, format!("base in {}", place))?;
        Ok(Self { memory, image, load, pod, user, base, ..self.to_owned() })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Vml(EngineVml),
    Ssh(EngineSsh),
    Docker(EngineDocker),
    Incus(EngineIncus),
    Podman(EnginePodman),
    Host,
    // Store any keys to ignore them
    Dbg(BTreeMap<String, Value>),
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
    pub engine: Option<Engine>,
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
    #[serde(default)]
    pub provide_workers: Vec<String>,
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
        result_fs_var: None,
        vars: Default::default(),
        export_vars: Default::default(),
        task_type,
        try_: None,
    };
    let taskset_elem = TasksetElem {
        requires: Default::default(),
        workers: default_taskset_elem_workers(),
        provide_workers: Default::default(),
        task,
    };
    BTreeMap::from([("Run taskline".to_owned(), taskset_elem)])
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtendVars {
    pub maps: Maps,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Extend {
    pub vars: ExtendVars,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    #[serde(default)]
    pub vars: Vars,
    #[serde(default)]
    pub extend: Extend,
    #[serde(default)]
    #[serde(rename = "use")]
    pub use_: Use,
    #[serde(default)]
    pub default: Defaults,
    #[serde(default)]
    pub networks: Networks,
    #[serde(default)]
    pub storages: Storages,
    #[serde(default)]
    pub workers: Workers,
    #[serde(default = "default_taskset")]
    pub taskset: Taskset,
    #[serde(default)]
    pub taskline: Vec<TasklineElem>,
    #[serde(default)]
    pub tasklines: Tasklines,
}

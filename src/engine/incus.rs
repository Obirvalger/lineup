use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};

use crate::cmd::Cmd;
use crate::engine::{EngineBase, ExistsAction};
use crate::manifest::EngineIncus as ManifestEngineIncus;
use crate::manifest::{EngineIncusNet, EngineIncusStorage};
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineIncus {
    pub memory: Option<String>,
    pub net: Option<EngineIncusNet>,
    pub nproc: Option<String>,
    pub image: String,
    pub storages: BTreeMap<String, EngineIncusStorage>,
    pub user: Option<String>,
    pub exists: ExistsAction,
    pub base: EngineBase,
    incus_bin: String,
}

impl EngineIncus {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_incus: &ManifestEngineIncus,
    ) -> Result<Self> {
        let manifest_engine_incus = manifest_engine_incus.render(context, "worker in manifest")?;
        let incus_bin = "incus".to_string();
        let nproc = manifest_engine_incus.nproc.map(|n| n.to_string());

        Ok(Self {
            memory: manifest_engine_incus.memory,
            net: manifest_engine_incus.net,
            nproc,
            image: manifest_engine_incus.image,
            storages: manifest_engine_incus.storages,
            user: manifest_engine_incus.user,
            exists: manifest_engine_incus.exists,
            base: manifest_engine_incus.base,
            incus_bin,
        })
    }

    pub fn start<S: AsRef<str>>(&self, name: S, action: &Option<ExistsAction>) -> Result<()> {
        let incus = self.incus_bin.to_string();
        let image = self.image.to_string();
        let name = self.n(name);

        let action = if let Some(action) = action { action } else { &self.exists };
        match action {
            ExistsAction::Fail => (),
            ExistsAction::Ignore => {
                let exists = run_fun!($incus ls -f json name=$name)?;
                if exists != "[]" {
                    let stopped = run_fun!($incus ls -f json status=stopped name=$name)?;
                    if stopped != "[]" {
                        run_fun!($incus start $name)?;
                    }
                    return Ok(());
                }
            }
            ExistsAction::Replace => {
                let exists = run_fun!($incus ls -f json name=$name)?;
                if exists != "[]" {
                    run_fun!($incus delete -qf $name)?;
                }
            }
        }

        run_fun!($incus init -q images:$image $name)?;

        if let Some(memory) = &self.memory {
            run_fun!(incus config set $name limits.memory=$memory)?;
        }
        if let Some(nproc) = &self.nproc {
            run_fun!(incus config set $name limits.cpu=$nproc)?;
        }

        if let Some(net) = &self.net {
            let device = &net.device;

            if let Some(network) = &net.network {
                run_fun!($incus network attach $network $name $device $device)?;
            }

            if let Some(address) = &net.address {
                run_fun!($incus config device set $name $device ipv4.address=$address)?;
            }
        }

        for (volume, storage) in &self.storages {
            let path = &storage.path;
            let pool = &storage.pool;

            let mut options = vec![];
            options.push(format!("pool={pool}"));
            options.push(format!("source={volume}"));
            if storage.readonly {
                options.push("readonly=true".to_string());
            }

            run_fun!($incus config device add -q $name $volume disk path=$path $[options])?;
        }

        run_fun!($incus start $name)?;
        Ok(())
    }

    pub fn restart<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let incus = &self.incus_bin;
        let name = self.n(name);

        run_fun!($incus stop $name)?;
        run_fun!($incus start $name)?;
        Ok(())
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let incus = self.incus_bin.to_string();
        let name = self.n(name);

        let exists = run_fun!($incus ls -f json name=$name)?;
        if exists != "[]" {
            run_fun!($incus rm -qf $name)?;
        }

        Ok(())
    }

    fn strip_same_name_dst<S: AsRef<Path>, D: AsRef<Path>>(src: S, dst: D) -> PathBuf {
        let src = src.as_ref();
        let dst = dst.as_ref();
        if let Some(src_name) = src.file_name() {
            if let Some(dst_name) = dst.file_name() {
                if src_name == dst_name {
                    let dst = dst.parent().expect("Destination has basename but lacks a dirname");
                    return dst.to_owned();
                }
            }
        }

        dst.to_owned()
    }

    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let mut dst = dst.as_ref().to_owned();
        let incus = self.incus_bin.to_string();
        let name = self.n(name);

        let mut options = vec![];
        if src.is_dir() {
            options.push("-r");
            // NOTE incus in resursive mode treats destination as target directory
            dst = Self::strip_same_name_dst(src, dst);
        }

        run_cmd!($incus file push $[options] $src $name/$dst)?;

        Ok(())
    }

    pub fn get<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let mut dst = dst.as_ref().to_owned();
        let incus = self.incus_bin.to_string();
        let name = self.n(name);

        let src_dir = run_fun!($incus exec $name -- test -d $src).is_ok();
        let mut options = vec![];
        if src_dir {
            options.push("-r");
            // NOTE incus in resursive mode treats destination as target directory
            dst = Self::strip_same_name_dst(src, dst);
        }

        run_cmd!($incus file pull $[options] $name/$src $dst)?;

        Ok(())
    }

    fn user_flags<N: AsRef<str>>(&self, name: N, cmd: &mut Cmd) {
        if let Some(user) = &self.user {
            let name = self.n(name);
            let err = "incus get id failed";
            let incus = self.incus_bin.to_string();

            let id = format!("echo $(id -g {0}):$(id -u {0})", user);
            let uid_gid = run_fun!($incus exec $name --user 65534 -- sh -c $id).expect(err);
            let (uid, gid) = uid_gid.split_once(':').expect(err);
            cmd.arg("--user");
            cmd.arg(uid);
            cmd.arg("--group");
            cmd.arg(gid);
        }
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        let mut cmd = Cmd::new(&self.incus_bin);
        cmd.arg("exec");
        cmd.arg(self.n(name.as_ref()));
        self.user_flags(name, &mut cmd);
        cmd.arg("--");
        cmd.args(["sh", "-c"]);
        cmd.arg(command.as_ref());

        cmd
    }

    pub fn exec_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, args: &[S]) -> Cmd {
        let mut cmd = Cmd::new(&self.incus_bin);
        cmd.arg("exec");
        cmd.arg(self.n(name.as_ref()));
        self.user_flags(name, &mut cmd);
        cmd.arg("--");
        cmd.args(args.iter().map(|a| a.as_ref()));

        cmd
    }

    fn n<S: AsRef<str>>(&self, name: S) -> String {
        self.base.name.to_owned().unwrap_or_else(|| name.as_ref().to_string())
    }
}

use std::path::Path;

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};

use crate::cmd::Cmd;
use crate::engine::{EngineBase, ExistsAction};
use crate::manifest::EngineIncus as ManifestEngineIncus;
use crate::manifest::EngineIncusNet;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineIncus {
    pub memory: Option<String>,
    pub net: Option<EngineIncusNet>,
    pub nproc: Option<String>,
    pub image: String,
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

        let mut options = vec!["-q".to_string()];

        if let Some(memory) = &self.memory {
            options.push("-c".to_string());
            options.push(format!("limits.memory={}", memory));
        }
        if let Some(nproc) = &self.nproc {
            options.push("-c".to_string());
            options.push(format!("limits.cpu={}", nproc));
        }

        if let Some(net) = &self.net {
            if let Some(network) = &net.network {
                options.push("--network".to_string());
                options.push(network.to_string());
            }

            if let Some(address) = &net.address {
                options.push("--device".to_string());
                options.push(format!("{},ipv4.address={}", &net.device, address));
            }
        }

        options.push(name.to_string());

        let action = if let Some(action) = action { action } else { &self.exists };
        match action {
            ExistsAction::Fail => (),
            ExistsAction::Ignore => {
                let exists = run_fun!($incus ls -f json $name)?;
                if exists != "[]" {
                    let stopped = run_fun!($incus ls status=stopped -f json $name)?;
                    if stopped != "[]" {
                        run_fun!($incus start $name)?;
                    }
                    return Ok(());
                }
            }
            ExistsAction::Replace => {
                run_fun!($incus delete -qf $name)?;
            }
        }

        run_fun!($incus launch images:$image $[options])?;
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

        let exists = run_fun!($incus ls -f json $name)?;
        if exists != "[]" {
            run_fun!($incus rm -qf $name)?;
        }

        Ok(())
    }

    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        let incus = self.incus_bin.to_string();
        let name = self.n(name);
        run_cmd!($incus file push $src $name/$dst)?;

        Ok(())
    }

    pub fn get<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        let incus = self.incus_bin.to_string();
        let name = self.n(name);
        run_cmd!($incus file pull $name/$src $dst)?;

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

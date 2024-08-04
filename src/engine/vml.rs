use std::path::Path;

use anyhow::Result;
use cmd_lib::run_cmd;

use crate::cmd::Cmd;
use crate::engine::{EngineBase, ExistsAction};
use crate::manifest::EngineVml as ManifestEngineVml;
use crate::manifest::{EngineVmlNet, EngineVmlNetTap};
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineVml {
    pub memory: Option<String>,
    pub nproc: Option<String>,
    pub image: Option<String>,
    pub parent: Option<String>,
    pub user: Option<String>,
    pub net: Option<EngineVmlNet>,
    pub exists: ExistsAction,
    pub base: EngineBase,
    vml_cmd: Vec<String>,
}

impl EngineVml {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_vml: &ManifestEngineVml,
    ) -> Result<Self> {
        let manifest_engine_vml = manifest_engine_vml.render(context, "worker in manifest")?;
        let vml_bin = manifest_engine_vml.vml_bin.unwrap_or_else(|| "vml".to_string());
        let nproc = manifest_engine_vml.nproc.map(|n| n.to_string());

        Ok(Self {
            memory: manifest_engine_vml.memory,
            image: manifest_engine_vml.image,
            net: manifest_engine_vml.net,
            nproc,
            parent: manifest_engine_vml.parent,
            user: manifest_engine_vml.user,
            exists: manifest_engine_vml.exists,
            base: manifest_engine_vml.base,
            vml_cmd: vec![vml_bin, "--log-level".to_string(), "error".to_string()],
        })
    }

    pub fn start<S: AsRef<str>>(&self, name: S, action: &Option<ExistsAction>) -> Result<()> {
        let vml = self.vml_cmd.to_owned();
        let name = self.n(name);

        let mut options = vec![];
        if let Some(memory) = &self.memory {
            options.push("--memory".to_string());
            options.push(memory.to_string());
        }
        if let Some(nproc) = &self.nproc {
            options.push("--nproc".to_string());
            options.push(nproc.to_string());
        }
        if let Some(image) = &self.image {
            options.push("--image".to_string());
            options.push(image.to_string());
        }
        if let Some(net) = &self.net {
            match net {
                EngineVmlNet::User => {
                    options.push("--net-user".to_string());
                }
                EngineVmlNet::Tap(EngineVmlNetTap { tap, address, gateway, nameservers }) => {
                    options.push("--net-tap".to_string());
                    options.push(tap.to_string());

                    if let Some(address) = address {
                        options.push("--net-address".to_string());
                        options.push(address.to_string());
                    }
                    if let Some(gateway) = gateway {
                        options.push("--net-gateway".to_string());
                        options.push(gateway.to_string());
                    }
                    if let Some(nameservers) = nameservers {
                        options.push("--net-nameservers".to_string());
                        options.append(&mut nameservers.to_owned());
                    }
                }
            }
        }

        let action = if let Some(action) = action { action } else { &self.exists };
        match action {
            ExistsAction::Fail => {
                options.push("--exists-fail".to_string());
                options.push("--running-fail".to_string());
            }
            ExistsAction::Ignore => {
                options.push("--exists-ignore".to_string());
                options.push("--running-ignore".to_string());
            }
            ExistsAction::Replace => {
                options.push("--exists-replace".to_string());
                options.push("--running-restart".to_string());
            }
        }

        run_cmd!($[vml] run $[options] --no-ssh -n $name)?;
        Ok(())
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let vml = self.vml_cmd.to_owned();
        let name = self.n(name);
        run_cmd!($[vml] rm -f -n $name)?;

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
        let vml = self.vml_cmd.to_owned();
        let name = self.n(name);

        let mut options = vec![];
        if let Some(user) = &self.user {
            options.push("--user");
            options.push(user);
        }

        run_cmd!($[vml] rsync-to $[options] -s $src -d $dst -n $name)?;

        Ok(())
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        let mut cmd = Cmd::from_args(&self.vml_cmd);
        cmd.args(["ssh", "--check"]);

        if let Some(user) = &self.user {
            cmd.arg("--user");
            cmd.arg(user);
        }

        let name = self.n(name);
        cmd.arg("-c");
        cmd.arg(command.as_ref());
        cmd.arg("-n");
        cmd.arg(name);

        cmd
    }

    fn n<S: AsRef<str>>(&self, name: S) -> String {
        let name = self.base.name.to_owned().unwrap_or_else(|| name.as_ref().to_string());
        if let Some(parent) = &self.parent {
            format!("{}/{}", parent, name)
        } else {
            name
        }
    }
}

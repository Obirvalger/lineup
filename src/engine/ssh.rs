use std::path::Path;

use anyhow::Result;
use cmd_lib::run_cmd;

use crate::cmd::Cmd;
use crate::engine::EngineBase;
use crate::manifest::EngineSsh as ManifestEngineSsh;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineSsh {
    pub host: String,
    pub port: Option<String>,
    pub user: Option<String>,
    pub key: Option<String>,
    pub ssh_cmd: Vec<String>,
    pub base: EngineBase,
}

impl EngineSsh {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_ssh: &ManifestEngineSsh,
    ) -> Result<Self> {
        let manifest_engine_ssh = manifest_engine_ssh.render(context, "worker in manifest")?;

        Ok(Self {
            host: manifest_engine_ssh.host,
            port: manifest_engine_ssh.port,
            user: manifest_engine_ssh.user,
            key: manifest_engine_ssh.key,
            ssh_cmd: manifest_engine_ssh.ssh_cmd,
            base: Default::default(),
        })
    }

    fn ssh_cmd(&self) -> Vec<String> {
        let mut ssh_cmd = self.ssh_cmd.to_owned();

        if let Some(key) = &self.key {
            ssh_cmd.push("-o".to_string());
            ssh_cmd.push("IdentitiesOnly=yes".to_string());
            ssh_cmd.push("-i".to_string());
            ssh_cmd.push(key.to_string());
        }

        if let Some(port) = &self.port {
            ssh_cmd.push("-p".to_string());
            ssh_cmd.push(port.to_string());
        }

        ssh_cmd
    }

    fn full_host(&self) -> String {
        if let Some(user) = &self.user {
            format!("{}@{}", user, &self.host)
        } else {
            self.host.to_owned()
        }
    }

    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        _name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let dst = format!("{}:{}", self.full_host(), dst.as_ref().display());
        let ssh_cmd = self.ssh_cmd().join(" ");

        run_cmd!(rsync -e $ssh_cmd -a $src $dst)?;

        Ok(())
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, _name: N, command: S) -> Cmd {
        let mut cmd = Cmd::from_args(self.ssh_cmd());
        cmd.arg(self.full_host());
        cmd.arg(command.as_ref());

        cmd
    }
}

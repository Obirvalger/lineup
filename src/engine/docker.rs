use std::path::{Path, PathBuf};

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};

use crate::cmd::Cmd;
use crate::engine::{EngineBase, ExistsAction};
use crate::manifest::EngineDocker as ManifestEngineDocker;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EngineDocker {
    pub memory: Option<String>,
    pub image: String,
    pub load: Option<PathBuf>,
    pub user: Option<String>,
    pub exists: ExistsAction,
    pub base: EngineBase,
    docker_bin: String,
    dir: PathBuf,
}

impl EngineDocker {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_docker: &ManifestEngineDocker,
        dir: &Path,
    ) -> Result<Self> {
        let manifest_engine_docker =
            manifest_engine_docker.render(context, "worker in manifest")?;
        let docker_bin = "docker".to_string();

        Ok(Self {
            memory: manifest_engine_docker.memory,
            image: manifest_engine_docker.image,
            load: manifest_engine_docker.load,
            user: manifest_engine_docker.user,
            exists: manifest_engine_docker.exists,
            base: manifest_engine_docker.base,
            docker_bin,
            dir: dir.to_owned(),
        })
    }

    pub fn start<S: AsRef<str>>(&self, name: S, action: &Option<ExistsAction>) -> Result<()> {
        let docker = self.docker_bin.to_string();
        let image = self.image.to_string();
        let name = self.n(name);

        if let Some(load) = &self.load {
            let load = if load.is_absolute() { load.to_owned() } else { self.dir.join(load) };
            run_fun!($docker load -qi $load)?;
        }

        let mut options = vec!["-dt".to_string()];
        if let Some(memory) = &self.memory {
            options.push("--memory".to_string());
            options.push(memory.to_string());
        }
        options.push("--name".to_string());
        options.push(name.to_string());

        let action = if let Some(action) = action { action } else { &self.exists };
        match action {
            ExistsAction::Fail => (),
            ExistsAction::Ignore => {
                if run_cmd!($docker container inspect -f "{{.Id}}" $name >/dev/null 2>&1).is_ok() {
                    let running =
                        run_fun!($docker container inspect -f "{{.State.Running}}" $name)?;
                    if running == "false" {
                        run_fun!($docker start $name)?;
                    }
                    return Ok(());
                }
            }
            ExistsAction::Replace => {
                run_fun!($docker rm -f $name)?;
            }
        }

        run_fun!($docker run $[options] $image)?;
        Ok(())
    }

    pub fn start_simple<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let docker = &self.docker_bin;
        let name = self.n(name);

        run_fun!($docker start $name)?;
        Ok(())
    }

    pub fn restart<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let docker = &self.docker_bin;
        let name = self.n(name);

        run_fun!($docker stop $name)?;
        run_fun!($docker start $name)?;
        Ok(())
    }

    pub fn stop<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let docker = &self.docker_bin;
        let name = self.n(name);

        run_fun!($docker stop $name)?;
        Ok(())
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let docker = self.docker_bin.to_string();
        let name = self.n(name);

        if run_cmd!($docker container inspect -f "{{.Id}}" $name >/dev/null 2>&1).is_ok() {
            run_fun!($docker rm -f $name)?;
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
        let docker = self.docker_bin.to_string();
        let name = self.n(name);
        run_cmd!($docker cp $src $name:$dst)?;

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
        let docker = self.docker_bin.to_string();
        let name = self.n(name);
        run_cmd!($docker cp $name:$src $dst)?;

        Ok(())
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        let mut cmd = Cmd::new(&self.docker_bin);
        cmd.args(["exec", "-i"]);

        if let Some(user) = &self.user {
            cmd.arg("--user");
            cmd.arg(user);
        }

        cmd.arg(self.n(name));
        cmd.args(["sh", "-c"]);
        cmd.arg(command.as_ref());

        cmd
    }

    fn n<S: AsRef<str>>(&self, name: S) -> String {
        self.base.name.to_owned().unwrap_or_else(|| name.as_ref().to_string())
    }
}

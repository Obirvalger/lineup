use std::path::{Path, PathBuf};

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};

use crate::cmd::Cmd;
use crate::engine::{EngineBase, ExistsAction};
use crate::manifest::EnginePodman as ManifestEnginePodman;
use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug)]
pub struct EnginePodman {
    pub memory: Option<String>,
    pub image: String,
    pub load: Option<PathBuf>,
    pub pod: Option<String>,
    pub user: Option<String>,
    pub exists: ExistsAction,
    pub base: EngineBase,
    podman_bin: String,
    dir: PathBuf,
}

impl EnginePodman {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine_podman: &ManifestEnginePodman,
        dir: &Path,
    ) -> Result<Self> {
        let manifest_engine_podman =
            manifest_engine_podman.render(context, "worker in manifest")?;
        let podman_bin = "podman".to_string();

        Ok(Self {
            memory: manifest_engine_podman.memory,
            image: manifest_engine_podman.image,
            load: manifest_engine_podman.load,
            pod: manifest_engine_podman.pod,
            user: manifest_engine_podman.user,
            exists: manifest_engine_podman.exists,
            base: manifest_engine_podman.base,
            podman_bin,
            dir: dir.to_owned(),
        })
    }

    pub fn start<S: AsRef<str>>(&self, name: S, action: &Option<ExistsAction>) -> Result<()> {
        let podman = self.podman_bin.to_string();
        let image = self.image.to_string();
        let name = self.n(name);

        if let Some(load) = &self.load {
            let load = if load.is_absolute() { load.to_owned() } else { self.dir.join(load) };
            run_fun!($podman load -qi $load)?;
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
                if run_cmd!($podman container exists $name).is_ok() {
                    let running = run_fun!(podman inspect -f "{{.State.Running}}" $name)?;
                    if running == "false" {
                        run_fun!($podman start $name)?;
                    }
                    return Ok(());
                }
            }
            ExistsAction::Replace => {
                options.push("--replace".to_string());
            }
        }

        if let Some(pod) = &self.pod {
            options.push("--pod".to_string());
            if run_cmd!($podman pod exists $pod).is_ok() {
                options.push(pod.to_string());
            } else {
                options.push(format!("new:{}", pod));
            }
        }
        run_fun!($podman run $[options] $image)?;
        Ok(())
    }

    pub fn restart<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let podman = &self.podman_bin;
        let name = self.n(name);

        run_fun!($podman stop $name)?;
        run_fun!($podman start $name)?;
        Ok(())
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let podman = self.podman_bin.to_string();
        let name = self.n(name);
        run_fun!($podman kill $name)?;
        run_fun!($podman rm -f $name)?;

        if let Some(pod) = &self.pod {
            if run_cmd!($podman pod exists $pod).is_ok()
                && run_fun!($podman pod inspect $pod --format "{{.NumContainers}}")? == "1"
            {
                run_fun!($podman pod rm $pod)?;
            }
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
        let podman = self.podman_bin.to_string();
        let name = self.n(name);
        run_cmd!($podman cp $src $name:$dst)?;

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
        let podman = self.podman_bin.to_string();
        let name = self.n(name);
        run_cmd!($podman cp $name:$src $dst)?;

        Ok(())
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        let mut cmd = Cmd::new(&self.podman_bin);
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

use std::path::Path;

use anyhow::{bail, Result};
use cmd_lib::run_fun;
use log::debug;
use serde::{Deserialize, Serialize};

pub use crate::engine::base::EngineBase;

use crate::cmd::{Cmd, CmdOut};
use crate::engine::dbg::EngineDbg;
use crate::engine::docker::EngineDocker;
use crate::engine::host::EngineHost;
use crate::engine::podman::EnginePodman;
use crate::engine::ssh::EngineSsh;
use crate::engine::vml::EngineVml;
use crate::error::Error;
use crate::manifest::Engine as ManifestEngine;
use crate::task_type::CmdParams;
use crate::template::Context;

mod base;
mod dbg;
mod docker;
mod host;
mod podman;
mod ssh;
mod vml;

#[derive(Clone, Debug)]
pub enum Engine {
    Dbg(EngineDbg),
    Docker(EngineDocker),
    Host(EngineHost),
    Podman(EnginePodman),
    Ssh(EngineSsh),
    Vml(EngineVml),
}

#[derive(clap::ValueEnum, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExistsAction {
    Fail,
    #[default]
    Ignore,
    Replace,
}

fn quote_args<S: AsRef<str>>(args: &[S]) -> Result<String> {
    let mut cmd = Vec::with_capacity(args.len());
    for arg in args {
        let arg = arg.as_ref();
        let quoted = run_fun!(printf %q $arg)?;
        cmd.push(quoted);
    }
    let command = cmd.join(" ");

    Ok(command)
}

impl Engine {
    pub fn from_manifest_engine(
        context: &Context,
        manifest_engine: &ManifestEngine,
    ) -> Result<Engine> {
        let engine = match manifest_engine {
            ManifestEngine::Dbg(_) => Engine::Dbg(EngineDbg { base: EngineBase::default() }),
            ManifestEngine::Docker(manifest_engine_docker) => Engine::Docker(
                EngineDocker::from_manifest_engine(context, manifest_engine_docker)?,
            ),
            ManifestEngine::Host => Engine::Host(EngineHost { base: EngineBase::default() }),
            ManifestEngine::Podman(manifest_engine_podman) => Engine::Podman(
                EnginePodman::from_manifest_engine(context, manifest_engine_podman)?,
            ),
            ManifestEngine::Ssh(manifest_engine_ssh) => {
                Engine::Ssh(EngineSsh::from_manifest_engine(context, manifest_engine_ssh)?)
            }
            ManifestEngine::Vml(manifest_engine_vml) => {
                Engine::Vml(EngineVml::from_manifest_engine(context, manifest_engine_vml)?)
            }
        };

        Ok(engine)
    }

    pub fn base(&self) -> &EngineBase {
        match self {
            Engine::Dbg(engine) => &engine.base,
            Engine::Docker(engine) => &engine.base,
            Engine::Host(engine) => &engine.base,
            Engine::Podman(engine) => &engine.base,
            Engine::Ssh(engine) => &engine.base,
            Engine::Vml(engine) => &engine.base,
        }
    }

    pub fn setup<S: AsRef<str>>(&self, name: S, action: &Option<ExistsAction>) -> Result<()> {
        if !self.base().setup {
            return Ok(());
        };
        match self {
            Engine::Dbg(_engine) => Ok(()),
            Engine::Docker(engine) => engine.start(name, action),
            Engine::Host(_engine) => Ok(()),
            Engine::Podman(engine) => engine.start(name, action),
            Engine::Ssh(_engine) => Ok(()),
            Engine::Vml(engine) => engine.start(name, action),
        }
    }

    pub fn remove<S: AsRef<str>>(&self, name: S) -> Result<()> {
        if !self.base().setup {
            return Ok(());
        };

        match self {
            Engine::Dbg(_engine) => Ok(()),
            Engine::Docker(engine) => engine.remove(name),
            Engine::Host(_engine) => Ok(()),
            Engine::Podman(engine) => engine.remove(name),
            Engine::Ssh(_engine) => Ok(()),
            Engine::Vml(engine) => engine.remove(name),
        }
    }

    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        match self {
            Engine::Dbg(engine) => engine.copy(name, src, dst),
            Engine::Docker(engine) => engine.copy(name, src, dst),
            Engine::Host(engine) => engine.copy(name, src, dst),
            Engine::Podman(engine) => engine.copy(name, src, dst),
            Engine::Ssh(engine) => engine.copy(name, src, dst),
            Engine::Vml(engine) => engine.copy(name, src, dst),
        }
    }

    pub fn get<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        match self {
            Engine::Dbg(engine) => engine.get(name, src, dst),
            Engine::Docker(engine) => engine.get(name, src, dst),
            Engine::Host(engine) => engine.get(name, src, dst),
            Engine::Podman(engine) => engine.get(name, src, dst),
            Engine::Ssh(engine) => engine.get(name, src, dst),
            Engine::Vml(engine) => engine.get(name, src, dst),
        }
    }

    fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        match self {
            Engine::Dbg(engine) => engine.shell_cmd(name, command),
            Engine::Docker(engine) => engine.shell_cmd(name, command),
            Engine::Host(engine) => engine.shell_cmd(name, command),
            Engine::Podman(engine) => engine.shell_cmd(name, command),
            Engine::Ssh(engine) => engine.shell_cmd(name, command),
            Engine::Vml(engine) => engine.shell_cmd(name, command),
        }
    }

    fn run<S: AsRef<str>>(
        &self,
        command_in_error: S,
        mut cmd: Cmd,
        params: &CmdParams,
    ) -> Result<()> {
        if let Some(stdin) = &params.stdin {
            cmd.set_stdin(stdin);
        }

        debug!("Run cmd: {}", cmd.get_args());
        let out = cmd.run()?;
        let stdout = out.stdout();
        let stderr = out.stderr();

        params.stdout.show(&stdout);
        params.stderr.show(&stderr);

        if params.check && !out.success(&params.success_codes) {
            bail!(Error::CommandFailedExitCode(command_in_error.as_ref().to_string()));
        }

        if let Some(matches) = &params.failure_matches {
            if matches.is_match(&stdout, &stderr)? {
                bail!(Error::CommandFailedFailureMatches(command_in_error.as_ref().to_string()));
            }
        }

        if let Some(matches) = &params.success_matches {
            if !matches.is_match(&stdout, &stderr)? {
                bail!(Error::CommandFailedSuccsessMatches(command_in_error.as_ref().to_string()));
            }
        }

        Ok(())
    }

    pub fn shell<N: AsRef<str>, S: AsRef<str>>(
        &self,
        name: N,
        command: S,
        params: &CmdParams,
    ) -> Result<()> {
        let cmd = self.shell_cmd(name, command.as_ref());

        self.run(command, cmd, params)
    }

    pub fn exec<N: AsRef<str>, S: AsRef<str>>(
        &self,
        name: N,
        args: &[S],
        params: &CmdParams,
    ) -> Result<()> {
        let command = quote_args(args)?;
        let cmd = match self {
            Engine::Dbg(engine) => engine.exec_cmd(name, args),
            Engine::Docker(engine) => engine.shell_cmd(name, &command),
            Engine::Host(engine) => engine.exec_cmd(name, args),
            Engine::Podman(engine) => engine.shell_cmd(name, &command),
            Engine::Ssh(engine) => engine.shell_cmd(name, &command),
            Engine::Vml(engine) => engine.shell_cmd(name, &command),
        };

        self.run(command, cmd, params)
    }

    pub fn shell_out<N: AsRef<str>, S: AsRef<str>>(
        &self,
        name: N,
        command: S,
        stdin: &Option<String>,
    ) -> Result<CmdOut> {
        let mut cmd = self.shell_cmd(name, command.as_ref());
        if let Some(stdin) = stdin {
            cmd.set_stdin(stdin);
        }

        cmd.run()
    }
}

use std::path::Path;

use anyhow::{bail, Result};
use cmd_lib::run_fun;
use log::debug;
use serde::{Deserialize, Serialize};

pub use crate::engine::base::EngineBase;

use crate::cmd::{Cmd, CmdOut};
use crate::config::CONFIG;
use crate::engine::dbg::EngineDbg;
use crate::engine::docker::EngineDocker;
use crate::engine::host::EngineHost;
use crate::engine::incus::EngineIncus;
use crate::engine::podman::EnginePodman;
use crate::engine::ssh::EngineSsh;
use crate::engine::vml::EngineVml;
use crate::error::Error;
use crate::manifest::Engine as ManifestEngine;
use crate::matches::Matches;
use crate::task_type::{CmdParams, SpecialTypeType};
use crate::template::Context;

mod base;
mod dbg;
mod docker;
mod host;
mod incus;
mod podman;
mod ssh;
mod vml;

#[derive(Clone, Debug)]
pub enum Engine {
    Dbg(EngineDbg),
    Docker(EngineDocker),
    Incus(EngineIncus),
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
        dir: &Path,
    ) -> Result<Engine> {
        let engine = match manifest_engine {
            ManifestEngine::Dbg(_) => Engine::Dbg(EngineDbg { base: EngineBase::default() }),
            ManifestEngine::Docker(manifest_engine_docker) => Engine::Docker(
                EngineDocker::from_manifest_engine(context, manifest_engine_docker, dir)?,
            ),
            ManifestEngine::Incus(manifest_engine_incus) => {
                Engine::Incus(EngineIncus::from_manifest_engine(context, manifest_engine_incus)?)
            }
            ManifestEngine::Host => Engine::Host(EngineHost { base: EngineBase::default() }),
            ManifestEngine::Podman(manifest_engine_podman) => Engine::Podman(
                EnginePodman::from_manifest_engine(context, manifest_engine_podman, dir)?,
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
            Engine::Incus(engine) => &engine.base,
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
            Engine::Incus(engine) => engine.start(name, action),
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
            Engine::Incus(engine) => engine.remove(name),
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
            Engine::Incus(engine) => engine.copy(name, src, dst),
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
            Engine::Incus(engine) => engine.get(name, src, dst),
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
            Engine::Incus(engine) => engine.shell_cmd(name, command),
            Engine::Host(engine) => engine.shell_cmd(name, command),
            Engine::Podman(engine) => engine.shell_cmd(name, command),
            Engine::Ssh(engine) => engine.shell_cmd(name, command),
            Engine::Vml(engine) => engine.shell_cmd(name, command),
        }
    }

    fn run_wrap_error(
        error: Error,
        matches: Option<&Matches>,
        params: &CmdParams,
        out: &CmdOut,
    ) -> Result<CmdOut> {
        let mut error_context = Vec::new();

        if let Some(stdin) = &params.stdin {
            error_context.push(("stdin", stdin.to_string()));
        }

        let stdout = out.stdout().trim_end().to_string();
        if !stdout.is_empty() || matches.is_some() {
            error_context.push(("stdout", stdout));
        }
        let stderr = out.stderr().trim_end().to_string();
        if !stderr.is_empty() || matches.is_some() {
            error_context.push(("stderr", stderr));
        }

        if let Some(matches) = matches {
            error_context.push((
                "matches",
                serde_json::to_string_pretty(matches).expect("Can't serialize matches"),
            ));
        }

        if let Some(rc) = out.rc() {
            if params.success_codes != [0] {
                error_context.push(("rc", rc.to_string()));
                error_context.push((
                    "success codes",
                    serde_json::to_string(&params.success_codes)
                        .expect("Can't serialize success codes"),
                ));
            } else if rc != 0 {
                error_context.push(("rc", rc.to_string()));
            }
        }

        error.result(error_context)
    }

    fn run<S: AsRef<str>>(
        &self,
        command_in_error: S,
        mut cmd: Cmd,
        params: &CmdParams,
    ) -> Result<CmdOut> {
        if let Some(stdin) = &params.stdin {
            cmd.set_stdin(stdin);
        }

        debug!("Run cmd: {}", cmd.get_args());
        let mut out = cmd.run()?;
        out.success_codes(&params.success_codes);
        let stdout = out.stdout();
        let stderr = out.stderr();

        params.stdout.show(&stdout);
        params.stderr.show(&stderr);

        if params.check.unwrap_or(CONFIG.task.command.check) && !out.success() {
            let error = Error::CommandFailedExitCode(command_in_error.as_ref().to_string());
            return Self::run_wrap_error(error, None, params, &out);
        }

        if let Some(matches) = &params.failure_matches {
            if matches.is_match(&stdout, &stderr)? {
                let error =
                    Error::CommandFailedFailureMatches(command_in_error.as_ref().to_string());
                return Self::run_wrap_error(error, Some(matches), params, &out);
            }
        }

        if let Some(matches) = &params.success_matches {
            if !matches.is_match(&stdout, &stderr)? {
                let error =
                    Error::CommandFailedSuccsessMatches(command_in_error.as_ref().to_string());

                return Self::run_wrap_error(error, Some(matches), params, &out);
            }
        }

        Ok(out)
    }

    pub fn shell<N: AsRef<str>, S: AsRef<str>>(
        &self,
        name: N,
        command: S,
        params: &CmdParams,
    ) -> Result<CmdOut> {
        let cmd = self.shell_cmd(name, command.as_ref());

        self.run(command, cmd, params)
    }

    pub fn exec<N: AsRef<str>, S: AsRef<str>>(
        &self,
        name: N,
        args: &[S],
        params: &CmdParams,
    ) -> Result<CmdOut> {
        let command = quote_args(args)?;
        let cmd = match self {
            Engine::Dbg(engine) => engine.exec_cmd(name, args),
            Engine::Docker(engine) => engine.shell_cmd(name, &command),
            Engine::Incus(engine) => engine.exec_cmd(name, args),
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

    pub fn special<N: AsRef<str>>(
        &self,
        name: N,
        type_: &SpecialTypeType,
        ignore_unsupported: bool,
    ) -> Result<()> {
        match type_ {
            SpecialTypeType::Restart => match self {
                Engine::Dbg(dbg) => dbg.restart(name)?,
                Engine::Docker(docker) => docker.restart(name)?,
                Engine::Incus(incus) => incus.restart(name)?,
                Engine::Podman(podman) => podman.restart(name)?,
                Engine::Vml(vml) => vml.restart(name)?,
                _ => {
                    if !ignore_unsupported {
                        bail!(Error::UnsupportedSpecialTask("restart".to_string(),))
                    }
                }
            },
        };

        Ok(())
    }
}

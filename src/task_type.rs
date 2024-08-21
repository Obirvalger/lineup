use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{log, LevelFilter};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::manifest::Tasklines;
use crate::matches::Matches;
use crate::module;
use crate::render::Render;
use crate::runner::Runner;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::tmpdir::tmpfile;
use crate::worker::Worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileTypeSource {
    #[serde(alias = "source")]
    Src(PathBuf),
    #[serde(alias = "contents")]
    Content(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileType {
    #[serde(alias = "dest")]
    #[serde(alias = "destination")]
    pub dst: PathBuf,
    #[serde(flatten)]
    pub source: FileTypeSource,
}

fn default_cmd_output_log() -> LevelFilter {
    LevelFilter::Off
}

fn default_cmd_output_print() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CmdOutput {
    #[serde(default = "default_cmd_output_log")]
    pub log: LevelFilter,
    #[serde(default = "default_cmd_output_print")]
    pub print: bool,
}

impl CmdOutput {
    pub fn show<S: AsRef<str>>(&self, output: S) {
        if let Some(level) = self.log.to_level() {
            for line in output.as_ref().lines() {
                log!(level.to_owned(), "{}", line)
            }
        }
        if self.print {
            print!("{}", output.as_ref());
        }
    }
}

fn default_cmd_stdout() -> CmdOutput {
    CmdOutput { log: LevelFilter::Trace, print: false }
}

fn default_cmd_stderr() -> CmdOutput {
    CmdOutput { log: LevelFilter::Warn, print: false }
}

fn default_cmd_check() -> bool {
    true
}

fn default_cmd_success_codes() -> Vec<i32> {
    vec![0]
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CmdParams {
    #[serde(default = "default_cmd_check")]
    pub check: bool,
    pub stdin: Option<String>,
    #[serde(default = "default_cmd_stdout")]
    pub stdout: CmdOutput,
    #[serde(default = "default_cmd_stderr")]
    pub stderr: CmdOutput,
    #[serde(default = "default_cmd_success_codes")]
    #[serde(alias = "sc")]
    pub success_codes: Vec<i32>,
    #[serde(alias = "sm")]
    pub success_matches: Option<Matches>,
    #[serde(alias = "fm")]
    pub failure_matches: Option<Matches>,
}

impl Render for CmdParams {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<CmdParams> {
        let stdin = self.stdin.render(context, format!("stdin in {}", place.as_ref()))?;
        let success_matches = self
            .success_matches
            .render(context, format!("success_matches in {}", place.as_ref()))?;
        let failure_matches = self
            .failure_matches
            .render(context, format!("failure_matches in {}", place.as_ref()))?;

        Ok(CmdParams { stdin, success_matches, failure_matches, ..self.to_owned() })
    }
}

impl Default for CmdParams {
    fn default() -> CmdParams {
        CmdParams {
            check: default_cmd_check(),
            stdin: Default::default(),
            stdout: default_cmd_stdout(),
            stderr: default_cmd_stderr(),
            success_codes: default_cmd_success_codes(),
            success_matches: Default::default(),
            failure_matches: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ExecType {
    args: Vec<String>,
    #[serde(flatten)]
    params: CmdParams,
}

impl ExecType {
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<()> {
        worker.exec(
            &self.args.render(context, "args in exec task")?,
            &self.params.render(context, "exec task")?,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ShellType {
    #[serde(alias = "cmd")]
    command: String,
    #[serde(flatten)]
    params: CmdParams,
}

impl ShellType {
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<()> {
        worker.shell(
            self.command.render(context, "command in shell task")?,
            &self.params.render(context, "shell task")?,
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunTasklineType {
    #[serde(default)]
    #[serde(alias = "tl")]
    taskline: String,
    #[serde(default)]
    module: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum TestTypeCommand {
    Exec(ExecType),
    Shell(ShellType),
}

impl TestTypeCommand {
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<()> {
        match self {
            Self::Exec(exec) => exec.run(context, worker),
            Self::Shell(shell) => shell.run(context, worker),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TestType {
    #[serde(alias = "cmds")]
    commands: Vec<TestTypeCommand>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    Exec(ExecType),
    Shell(ShellType),
    File(FileType),
    RunTaskline(RunTasklineType),
    Run(String),
    Test(TestType),
}

impl TaskType {
    pub fn run(
        &self,
        context: &Context,
        dir: &Path,
        tasklines: &Tasklines,
        worker: &Worker,
    ) -> Result<()> {
        let mut context = context.to_owned();
        match self {
            Self::Exec(exec) => exec.run(&context, worker),
            Self::Shell(shell) => shell.run(&context, worker),
            Self::File(FileType { dst, source }) => {
                let dst = dst.render(&context, "file task dst")?;
                match source {
                    FileTypeSource::Src(src) => {
                        worker.copy(src.render(&context, "file task src")?, dst)
                    }
                    FileTypeSource::Content(contents) => {
                        let src = tmpfile();
                        fs::write(&src, contents)?;
                        worker.copy(src, dst)
                    }
                }
            }
            Self::Run(taskline) => Self::RunTaskline(RunTasklineType {
                taskline: taskline.to_owned(),
                module: Default::default(),
            })
            .run(&context, dir, tasklines, worker),
            Self::RunTaskline(RunTasklineType { taskline, module }) => {
                let module = module.render(&context, "run-taskline file")?;
                let taskline = taskline.render(&context, "run-taskline taskline")?;
                let mut dir = dir.to_owned();
                let mut new_tasklines = tasklines.to_owned();
                let mut taskline = if module.display().to_string().is_empty() {
                    tasklines
                        .get(&taskline)
                        .ok_or(Error::BadTaskline(taskline.to_string(), PathBuf::from("")))?
                        .to_owned()
                } else {
                    let file = module::resolve(&module, &dir);
                    Taskline::File { file, name: taskline }
                };

                while !taskline.is_line() {
                    match &taskline {
                        Taskline::File { file, name } => {
                            let runner = Runner::from_manifest(file)?;
                            runner.dir.clone_into(&mut dir);
                            runner.tasklines.clone_into(&mut new_tasklines);
                            let mut new_context = runner.vars.context()?;
                            new_context.extend(context);
                            context = new_context;
                            runner
                                .tasklines
                                .get(name)
                                .ok_or(Error::BadTaskline(name.to_string(), file.to_owned()))?
                                .clone_into(&mut taskline)
                        }
                        Taskline::Line(_) => break,
                    }
                }

                for task in taskline.as_line().expect("get not line variant of taskline") {
                    task.task.run(&task.name, &context, &dir, &new_tasklines, worker)?;
                }

                Ok(())
            }
            Self::Test(TestType { commands }) => {
                for command in commands {
                    command.run(&context, worker)?;
                }

                Ok(())
            }
        }
    }
}

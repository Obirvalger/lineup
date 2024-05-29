use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{log, LevelFilter};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::config_dir;
use crate::error::Error;
use crate::manifest::Tasklines;
use crate::matches::Matches;
use crate::render::Render;
use crate::runner::Runner;
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
pub struct CommandType {
    args: Vec<String>,
    #[serde(flatten)]
    params: CmdParams,
}

impl CommandType {
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<()> {
        worker.exec(
            &self.args.render(context, "args in command task")?,
            &self.params.render(context, "command task")?,
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

fn deserialize_run_taskline_type_source<'de, D>(
    deserializer: D,
) -> Result<RunTasklineTypeSource, D::Error>
where
    D: Deserializer<'de>,
{
    let source = Option::<RunTasklineTypeSource>::deserialize(deserializer)?;
    Ok(source.unwrap_or(RunTasklineTypeSource::default()))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunTasklineTypeSource {
    File(PathBuf),
    Module(PathBuf),
}

impl Default for RunTasklineTypeSource {
    fn default() -> Self {
        RunTasklineTypeSource::File(Default::default())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunTasklineType {
    #[serde(default)]
    #[serde(alias = "tl")]
    taskline: String,
    #[serde(flatten, deserialize_with = "deserialize_run_taskline_type_source")]
    source: RunTasklineTypeSource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum TestTypeCommand {
    Command(CommandType),
    Shell(ShellType),
}

impl TestTypeCommand {
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<()> {
        match self {
            Self::Command(command) => command.run(context, worker),
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
    Command(CommandType),
    Shell(ShellType),
    File(FileType),
    RunTaskline(RunTasklineType),
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
            Self::Command(command) => command.run(&context, worker),
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
            Self::RunTaskline(RunTasklineType { taskline, source }) => {
                let file = match source {
                    RunTasklineTypeSource::File(file) => file.to_owned(),
                    RunTasklineTypeSource::Module(module) => {
                        config_dir().join("modules").join(module).with_extension("toml")
                    }
                };
                let file = file.render(&context, "run-taskline file")?;
                let taskline = taskline.render(&context, "run-taskline taskline")?;
                let mut dir = dir.to_owned();
                let mut new_tasklines = tasklines.to_owned();
                let taskline = if file.display().to_string().is_empty() {
                    tasklines
                        .get(&taskline)
                        .ok_or(Error::BadTaskline(taskline.to_string(), PathBuf::from("")))?
                        .to_owned()
                } else {
                    let mut file = PathBuf::from(&file);
                    if !file.is_absolute() {
                        file = dir.join(file);
                    }
                    let runner = Runner::from_manifest(&file)?;
                    dir = runner.dir.to_owned();
                    new_tasklines = runner.tasklines.to_owned();
                    let mut new_context = runner.vars.context()?;
                    new_context.extend(context);
                    context = new_context;
                    runner
                        .tasklines
                        .get(&taskline)
                        .ok_or(Error::BadTaskline(taskline, file))?
                        .to_owned()
                };

                for task in taskline {
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

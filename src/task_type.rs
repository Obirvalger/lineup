use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{log, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cmd::CmdOut;
use crate::error::Error;
use crate::manifest::Tasklines;
use crate::matches::Matches;
use crate::module;
use crate::render::Render;
use crate::runner::Runner;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::tmpdir::tmpfile;
use crate::vars::Var;
use crate::worker::Worker;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnsureType {
    #[serde(default)]
    pub vars: Vec<Var>,
}

impl EnsureType {
    fn ensure_vars(&self, context: &Context) -> Result<()> {
        let mut absent_vars = vec![];

        'vars: for var in &self.vars {
            if !var.kind.is_nothing() {
                warn!("kind doest not ensures on variable `{}`", var)
            }

            let mut value = context.to_owned().into_json();
            for part in var.name.split('.') {
                match value.get(part) {
                    Some(new_value) => value = new_value.to_owned(),
                    None => {
                        absent_vars.push(var.to_string());
                        continue 'vars;
                    }
                }
            }

            var.check_type(&value)?;
        }

        if !absent_vars.is_empty() {
            let mut taskline = "".to_string();
            if let Some(taskline_str) = context.get("taskline").and_then(|t| t.as_str()) {
                taskline = taskline_str.to_string();
            } else {
                warn!("taskline absent in context for EnsureType");
            }
            bail!(Error::EnsureAbsentVars(absent_vars.join(" "), taskline))
        }

        Ok(())
    }

    pub fn ensure(&self, context: &Context) -> Result<Value> {
        self.ensure_vars(context)?;

        Ok(Value::Bool(true))
    }
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GetType {
    #[serde(alias = "source")]
    pub src: PathBuf,
    #[serde(alias = "dest")]
    #[serde(alias = "destination")]
    pub dst: Option<PathBuf>,
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
    pub fn run_out(&self, context: &Context, worker: &Worker) -> Result<CmdOut> {
        worker.exec(
            &self.args.render(context, "args in exec task")?,
            &self.params.render(context, "exec task")?,
        )
    }

    pub fn run(&self, context: &Context, worker: &Worker) -> Result<Value> {
        let out = self.run_out(context, worker)?;
        Ok(Value::String(out.stdout()))
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
    pub fn run_out(&self, context: &Context, worker: &Worker) -> Result<CmdOut> {
        worker.shell(
            self.command.render(context, "command in shell task")?,
            &self.params.render(context, "shell task")?,
        )
    }

    pub fn run(&self, context: &Context, worker: &Worker) -> Result<Value> {
        let out = self.run_out(context, worker)?;
        Ok(Value::String(out.stdout()))
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
    pub fn run(&self, context: &Context, worker: &Worker) -> Result<CmdOut> {
        match self {
            Self::Exec(exec) => exec.run_out(context, worker),
            Self::Shell(shell) => shell.run_out(context, worker),
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
    Ensure(EnsureType),
    Exec(ExecType),
    Shell(ShellType),
    File(FileType),
    Get(GetType),
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
    ) -> Result<Value> {
        let mut context = context.to_owned();
        match self {
            Self::Ensure(ensure) => ensure.ensure(&context),
            Self::Exec(exec) => exec.run(&context, worker),
            Self::Shell(shell) => shell.run(&context, worker),
            Self::File(FileType { dst, source }) => {
                let dst = dst.render(&context, "file task dst")?;
                match source {
                    FileTypeSource::Src(src) => {
                        worker.copy(src.render(&context, "file task src")?, &dst)
                    }
                    FileTypeSource::Content(contents) => {
                        let src = tmpfile();
                        fs::write(&src, contents)?;
                        worker.copy(src, &dst)
                    }
                }?;

                Ok(Value::String(dst.to_string_lossy().to_string()))
            }
            Self::Get(GetType { src, dst }) => {
                let src = src.render(&context, "get task src")?;
                let dst = if let Some(dst) = dst {
                    dst.render(&context, "get task dst")?
                } else {
                    let name =
                        src.file_name().ok_or_else(|| Error::GetSrcFilename(src.to_owned()))?;
                    dir.join(name)
                };
                worker.get(src, &dst)?;

                Ok(Value::String(dst.to_string_lossy().to_string()))
            }
            Self::Run(taskline) => Self::RunTaskline(RunTasklineType {
                taskline: taskline.to_owned(),
                module: Default::default(),
            })
            .run(&context, dir, tasklines, worker),
            Self::RunTaskline(RunTasklineType { taskline, module }) => {
                let module = module.render(&context, "run-taskline file")?;
                let taskline_name = taskline.render(&context, "run-taskline taskline")?;
                let mut taskline_file = "".to_string();
                let mut dir = dir.to_owned();
                let mut new_tasklines = tasklines.to_owned();
                let mut taskline = if module.display().to_string().is_empty() {
                    tasklines
                        .get(&taskline_name)
                        .ok_or(Error::BadTaskline(taskline.to_string(), PathBuf::from("")))?
                        .to_owned()
                } else {
                    let file = module::resolve(&module, &dir);
                    taskline_file = file.display().to_string();
                    Taskline::File { file, name: taskline_name.to_string() }
                };

                while !taskline.is_line() {
                    match &taskline {
                        Taskline::File { file, name } => {
                            let runner = Runner::from_manifest(file, &context)?;
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

                let taskline_str = if taskline_file.is_empty() {
                    taskline_name
                } else if taskline_name.is_empty() {
                    taskline_file
                } else {
                    format!("{}:{}", taskline_file, taskline_name)
                };
                context.insert("taskline", &taskline_str);

                let mut value = Value::Null;
                for task in taskline.as_line().expect("get not line variant of taskline") {
                    value = task.task.run(&task.name, &context, &dir, &new_tasklines, worker)?;
                    context.insert("result", &value);
                }

                Ok(value)
            }
            Self::Test(TestType { commands }) => {
                let mut success = true;

                for command in commands {
                    success &= command.run(&context, worker)?.success();
                }

                Ok(Value::Bool(success))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::Value;

    fn context() -> Context {
        let mut context = Context::new();
        context.insert("user", "user");
        context.insert("packages", &["apt-repo"]);
        let vars: Value = serde_json::from_str(r#"{"one": 1}"#).unwrap();
        context.insert("vars", &vars);
        let out: Value = serde_json::from_str(r#"{"in": {"one": 1}}"#).unwrap();
        context.insert("out", &out);

        context
    }

    #[test]
    fn empty_ensure_vars_empty_context() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = Default::default();
        ensure.ensure_vars(&Context::new())
    }

    #[test]
    fn empty_ensure_vars() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = Default::default();
        ensure.ensure_vars(&context())
    }

    #[test]
    fn non_nested_ensure_vars() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["user".parse()?, "packages".parse()?];
        ensure.ensure_vars(&context())
    }

    #[test]
    fn non_nested_ensure_vars_absent() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["target".parse()?];
        assert!(ensure.ensure_vars(&context()).is_err());

        Ok(())
    }

    #[test]
    fn nested_ensure_vars() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["vars.one".parse()?, "out.in.one".parse()?];
        ensure.ensure_vars(&context())
    }

    #[test]
    fn nested_ensure_vars_absent() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["out.in.two".parse()?];
        assert!(ensure.ensure_vars(&context()).is_err());

        Ok(())
    }

    #[test]
    fn top_level_ensure_vars() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["vars".parse()?, "out.in".parse()?];
        ensure.ensure_vars(&context())
    }

    #[test]
    fn top_level_ensure_vars_absent() -> Result<()> {
        let mut ensure = EnsureType::default();
        ensure.vars = vec!["out.vars".parse()?];
        assert!(ensure.ensure_vars(&context()).is_err());

        Ok(())
    }
}

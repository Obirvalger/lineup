use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use log::{debug, info, log, trace, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cmd::CmdOut;
use crate::config::CONFIG;
use crate::error::Error;
use crate::exception::Exception;
use crate::manifest::Tasklines;
use crate::matches::Matches;
use crate::module;
use crate::quote::quote;
use crate::render::Render;
use crate::runner::Runner;
use crate::task_result::TaskResult;
use crate::taskline::Taskline;
use crate::template::Context;
use crate::vars::{Var, Vars};
use crate::worker::Worker;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BreakType {
    #[serde(default)]
    pub taskline: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct DebugType {
    msg: String,
    #[serde(default)]
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DummyType {
    #[serde(default)]
    pub result: Option<Value>,
}

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
            bail!(Error::EnsureAbsentVars(absent_vars.join(", "), taskline))
        }

        Ok(())
    }

    pub fn ensure(&self, context: &Context) -> Result<Value> {
        self.ensure_vars(context)?;

        Ok(Value::Bool(true))
    }
}

fn default_error_code() -> i32 {
    1
}

fn default_error_trace() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ErrorType {
    msg: String,
    #[serde(default = "default_error_code")]
    code: i32,
    #[serde(default = "default_error_trace")]
    trace: bool,
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
    pub chown: Option<String>,
    pub chmod: Option<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct InfoType {
    msg: String,
    #[serde(default)]
    pub result: Option<Value>,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CmdParamsResultStream {
    #[default]
    Stdout,
    Stderr,
}

fn default_cmd_params_result_lines() -> bool {
    true
}

fn default_cmd_params_result_strip() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct CmdParamsResult {
    #[serde(default = "default_cmd_params_result_lines")]
    lines: bool,
    #[serde(default)]
    matched: bool,
    #[serde(default)]
    #[serde(alias = "rc")]
    return_code: bool,
    #[serde(default)]
    stream: CmdParamsResultStream,
    #[serde(default = "default_cmd_params_result_strip")]
    strip: bool,
}

impl CmdParamsResult {
    fn get(&self, out: CmdOut) -> Value {
        if self.return_code {
            return out.rc().map(|c| c.into()).unwrap_or(Value::Null);
        }

        if self.matched {
            return Value::Bool(out.matched);
        }

        let mut result = match self.stream {
            CmdParamsResultStream::Stdout => out.stdout(),
            CmdParamsResultStream::Stderr => out.stderr(),
        };

        if self.strip {
            result = result.trim_end().to_string();
        }

        if self.lines {
            let a = result.lines().map(|l| Value::String(l.to_string())).collect::<Vec<_>>();
            Value::Array(a)
        } else {
            Value::String(result)
        }
    }
}

impl Default for CmdParamsResult {
    fn default() -> Self {
        Self {
            lines: default_cmd_params_result_lines(),
            matched: Default::default(),
            return_code: Default::default(),
            stream: Default::default(),
            strip: default_cmd_params_result_strip(),
        }
    }
}

fn default_cmd_stdout() -> CmdOutput {
    CONFIG.task.command.stdout.to_owned()
}

fn default_cmd_stderr() -> CmdOutput {
    CONFIG.task.command.stderr.to_owned()
}

fn default_cmd_check() -> bool {
    CONFIG.task.command.check
}

fn default_cmd_success_codes() -> Vec<i32> {
    vec![0]
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CmdParams {
    pub check: Option<bool>,
    #[serde(default)]
    pub result: CmdParamsResult,
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

impl CmdParams {
    pub fn quiet() -> Self {
        let mut cmd_params = CmdParams::default();
        let cmd_output = CmdOutput { log: LevelFilter::Off, print: false };
        cmd_params.stderr = cmd_output.to_owned();
        cmd_params.stdout = cmd_output.to_owned();

        cmd_params
    }
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
            check: Default::default(),
            result: Default::default(),
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
    pub fn run_out(&self, context: &Context, worker: &Worker, check: bool) -> Result<CmdOut> {
        let mut params = self.params.render(context, "exec task")?;
        params.check.get_or_insert(check);
        worker.exec(&self.args.render(context, "args in exec task")?, &params)
    }

    pub fn run(&self, context: &Context, worker: &Worker) -> Result<Value> {
        let out = self.run_out(context, worker, default_cmd_check())?;
        Ok(self.params.result.get(out))
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
pub enum RunTasksetTypeWorker {
    All,
    Maps(Vec<(String, String)>),
    Names(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct RunTasksetType {
    module: PathBuf,
    worker: RunTasksetTypeWorker,
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
    pub fn run_out(&self, context: &Context, worker: &Worker, check: bool) -> Result<CmdOut> {
        let mut params = self.params.render(context, "shell task")?;
        params.check.get_or_insert(check);
        worker.shell(self.command.render(context, "command in shell task")?, &params)
    }

    pub fn run(&self, context: &Context, worker: &Worker) -> Result<Value> {
        let out = self.run_out(context, worker, default_cmd_check())?;
        Ok(self.params.result.get(out))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecialTypeType {
    Restart,
}

fn default_special_ignore_unsupported() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecialType {
    #[serde(default = "default_special_ignore_unsupported")]
    ignore_unsupported: bool,
    #[serde(flatten)]
    type_: SpecialTypeType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum TestTypeCommand {
    Exec(ExecType),
    ExecArgs(Vec<String>),
    Shell(ShellType),
    ShellCommand(String),
}

impl TestTypeCommand {
    pub fn run(&self, context: &Context, worker: &Worker, check: bool) -> Result<CmdOut> {
        match self {
            Self::Exec(exec) => exec.run_out(context, worker, check),
            Self::ExecArgs(args) => {
                let exec = ExecType { args: args.to_owned(), params: Default::default() };
                exec.run_out(context, worker, check)
            }
            Self::Shell(shell) => shell.run_out(context, worker, check),
            Self::ShellCommand(command) => {
                let shell = ShellType { command: command.to_string(), params: Default::default() };
                shell.run_out(context, worker, check)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TestType {
    #[serde(alias = "cmds")]
    commands: Vec<TestTypeCommand>,
    #[serde(default = "default_cmd_check")]
    check: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TraceType {
    msg: String,
    #[serde(default)]
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct WarnType {
    msg: String,
    #[serde(default)]
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    Break(BreakType),
    Debug(DebugType),
    Dummy(DummyType),
    Ensure(EnsureType),
    Error(ErrorType),
    Exec(ExecType),
    File(FileType),
    Get(GetType),
    Info(InfoType),
    Run(String),
    RunTaskline(RunTasklineType),
    RunTaskset(RunTasksetType),
    Shell(ShellType),
    Special(SpecialType),
    Test(TestType),
    Trace(TraceType),
    Warn(WarnType),
}

impl TaskType {
    pub fn run(
        &self,
        context: &Context,
        dir: &Path,
        tasklines: &Tasklines,
        workers: &[Worker],
        worker: &Worker,
    ) -> Result<TaskResult> {
        let mut context = context.to_owned();
        match self {
            Self::Break(BreakType { taskline, result }) => {
                let result = if let Some(result) = result {
                    result.render(&context, "break result")?
                } else {
                    context.get("result").cloned().unwrap_or(Value::Null)
                };
                Ok(Exception::BreakTaskline {
                    taskline: taskline.render(&context, "break taskline")?,
                    result,
                }
                .into())
            }
            Self::Debug(DebugType { msg, result }) => {
                let msg = msg.render(&context, "debug msg")?;
                debug!("{}", msg);
                if let Some(result) = result {
                    result.render(&context, "debug result").map(|ok| ok.into())
                } else {
                    Ok(context.get("result").cloned().unwrap_or(Value::Null).into())
                }
            }
            Self::Dummy(dummy) => {
                if let Some(result) = &dummy.result {
                    result.render(&context, "dummy result").map(|ok| ok.into())
                } else {
                    Ok(context.get("result").cloned().unwrap_or(Value::Null).into())
                }
            }
            Self::Ensure(ensure) => ensure.ensure(&context).map(|ok| ok.into()),
            Self::Error(ErrorType { msg, code, trace }) => {
                let msg = msg.render(&context, "error msg")?;
                bail!(Error::User(msg, *code, *trace));
            }
            Self::Exec(exec) => exec.run(&context, worker).map(|ok| ok.into()),
            Self::File(FileType { dst, source, chown, chmod }) => {
                let dst = dst.render(&context, "file task dst")?;
                match source {
                    FileTypeSource::Src(src) => {
                        worker.copy(src.render(&context, "file task src")?, &dst)
                    }
                    FileTypeSource::Content(contents) => {
                        let contents = contents.render(&context, "file task contents")?;
                        let dst_quoted = quote(dst.to_string_lossy())?;
                        let mut cmd_params = CmdParams::quiet();
                        cmd_params.stdin = Some(contents);
                        worker.shell(format!("cat > {dst_quoted}"), &cmd_params)?;
                        Ok(())
                    }
                }?;

                if let Some(chown) = chown {
                    worker.exec(
                        &["chown", "-R", chown, &dst.to_string_lossy()],
                        &CmdParams::quiet(),
                    )?;
                }

                if let Some(chmod) = chmod {
                    worker.exec(
                        &["chmod", "-R", chmod, &dst.to_string_lossy()],
                        &CmdParams::quiet(),
                    )?;
                }

                Ok(Value::String(dst.to_string_lossy().to_string()).into())
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

                Ok(Value::String(dst.to_string_lossy().to_string()).into())
            }
            Self::Info(InfoType { msg, result }) => {
                let msg = msg.render(&context, "info msg")?;
                info!("{}", msg);
                if let Some(result) = result {
                    result.render(&context, "info result").map(|ok| ok.into())
                } else {
                    Ok(context.get("result").cloned().unwrap_or(Value::Null).into())
                }
            }
            Self::Run(taskline) => Self::RunTaskline(RunTasklineType {
                taskline: taskline.to_owned(),
                module: Default::default(),
            })
            .run(&context, dir, tasklines, workers, worker),
            Self::RunTaskline(RunTasklineType { taskline: taskline_name, module }) => {
                let module = module.render(&context, "run-taskline file")?;
                let taskline_name = taskline_name.render(&context, "run-taskline taskline")?;
                let mut taskline_file = "".to_string();
                let mut dir = dir.to_owned();
                let mut new_tasklines = tasklines.to_owned();
                let mut taskline = if module.display().to_string().is_empty() {
                    tasklines
                        .get(&taskline_name)
                        .ok_or(Error::BadTaskline(taskline_name.to_string(), PathBuf::from("")))?
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
                for (iter, task) in taskline
                    .as_line()
                    .expect("get not line variant of taskline")
                    .iter()
                    .enumerate()
                {
                    let result = task
                        .task
                        .run(&task.name, &context, &dir, &new_tasklines, workers, worker)
                        .with_context(|| {
                            format!("taskline: `{}`, number: `{}`", taskline_str, iter)
                        })?;

                    if let Some(v) = result.as_value() {
                        if let Some(vars_context) = result.as_context() {
                            context.extend(vars_context);
                        }
                        value = v.to_owned();
                        context.insert("result", &value);
                    } else if let Some(exception) = result.as_exception() {
                        match exception {
                            Exception::BreakTaskline { taskline, result } => {
                                let break_taskline = taskline.as_ref().unwrap_or(&taskline_str);
                                if break_taskline == &taskline_str {
                                    return Ok(result.to_owned().into());
                                } else {
                                    return Ok(exception.to_owned().into());
                                }
                            }
                        }
                    }
                }

                Ok(value.into())
            }
            Self::RunTaskset(RunTasksetType { module, worker }) => {
                let module = module.render(&context, "run-taskline file")?;
                let file = module::resolve(&module, dir);
                let new_workers = match worker {
                    RunTasksetTypeWorker::All => workers.to_owned(),
                    RunTasksetTypeWorker::Maps(maps) => {
                        let maps = maps.render(&context, "run-taskset maps")?;
                        let mut new_workers = vec![];
                        for worker in workers {
                            for map in &maps {
                                if map.0 == worker.name() {
                                    let mut new_worker = worker.to_owned();
                                    new_worker.rename(&map.1);
                                    new_workers.push(new_worker);
                                }
                            }
                        }
                        new_workers
                    }
                    RunTasksetTypeWorker::Names(names) => {
                        let names = names.render(&context, "run-taskset maps")?;
                        workers
                            .iter()
                            .filter(|w| names.contains(&w.name()))
                            .map(|w| w.to_owned())
                            .collect()
                    }
                };

                let mut runner = Runner::from_manifest(file, &context)?;
                runner.add_extra_vars(Vars::from(context.to_owned()));
                runner.set_workers(&new_workers);
                runner.run()?;
                Ok(Value::Null.into())
            }
            Self::Shell(shell) => shell.run(&context, worker).map(|ok| ok.into()),
            Self::Special(SpecialType { type_, ignore_unsupported }) => {
                worker.special(type_, *ignore_unsupported)?;
                Ok(Value::Null.into())
            }
            Self::Test(TestType { commands, check }) => {
                let mut success = true;

                for command in commands {
                    success &= command.run(&context, worker, *check)?.success();
                }

                Ok(Value::Bool(success).into())
            }
            Self::Trace(TraceType { msg, result }) => {
                let msg = msg.render(&context, "trace msg")?;
                trace!("{}", msg);
                if let Some(result) = result {
                    result.render(&context, "trace result").map(|ok| ok.into())
                } else {
                    Ok(context.get("result").cloned().unwrap_or(Value::Null).into())
                }
            }
            Self::Warn(WarnType { msg, result }) => {
                let msg = msg.render(&context, "warn msg")?;
                warn!("{}", msg);
                if let Some(result) = result {
                    result.render(&context, "warn result").map(|ok| ok.into())
                } else {
                    Ok(context.get("result").cloned().unwrap_or(Value::Null).into())
                }
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

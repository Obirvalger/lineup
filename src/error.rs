use std::path::PathBuf;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("extra var `{0}` does not have '=' to delimit name")]
    BadExtraVar(String),
    #[error("Fs var name should be alphanumeric, but get `{0}`")]
    BadFsVar(String),
    #[error("kind argument `{0}` does not have ':' to delimit name")]
    BadKindArg(String),
    #[error("kind argument `render` must be true or false, but get `{0}`")]
    BadKindArgRedner(String),
    #[error("bad path to manifest `{0}`")]
    BadManifest(PathBuf),
    #[error("failed to get taskline `{0}` from file `{1}`")]
    BadTaskline(String, PathBuf),
    #[error("failed to get task `{0}` from taskset")]
    BadTaskInTaskset(String),
    #[error("could not parse variable `{0}`")]
    BadVar(String),
    #[error("child process stdin has not been captured")]
    ChildStdin,
    #[error("command `{0}` failed: return failure exit code")]
    CommandFailedExitCode(String),
    #[error("command `{0}` failed: match failure matches")]
    CommandFailedFailureMatches(String),
    #[error("command `{0}` failed: don't match success matches")]
    CommandFailedSuccsessMatches(String),
    #[error("variables `{0}` are not set for taskline `{1}`")]
    EnsureAbsentVars(String, String),
    #[error("get task's src `{0}` has no filename")]
    GetSrcFilename(PathBuf),
    #[error("no engine provided to worker `{0}`")]
    NoEngine(String),
    #[error("failed tsort in {0}")]
    TSort(String),
    #[error("unknown variable kind `{0}`")]
    UnknownVarKind(String),
    #[error("unknown variable type `{0}`")]
    UnknownVarType(String),
    #[error("special task `{0}` does not work on this engine")]
    UnsupportedSpecialTask(String),
    #[error("cannot use tasklines `{0}` from the `{1}`")]
    UseTasklines(String, PathBuf),
    #[error("cannot use vars `{0}` from the `{1}`")]
    UseVars(String, PathBuf),
    #[error("failed to setup worker `{0}`")]
    WorkerSetupFailed(String),
    #[error("variable `{0}` must be of type `{1}`")]
    WrongVarType(String, String),
}

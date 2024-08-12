use std::path::PathBuf;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("extra var `{0}` does not have '=' to delimit name")]
    BadExtraVar(String),
    #[error("bad path to manifest `{0}`")]
    BadManifest(PathBuf),
    #[error("failed to get taskline `{0}` from file `{1}`")]
    BadTaskline(String, PathBuf),
    #[error("failed to get task `{0}` from taskset")]
    BadTaskInTaskset(String),
    #[error("child process stdin has not been captured")]
    ChildStdin,
    #[error("command `{0}` failed: return failure exit code")]
    CommandFailedExitCode(String),
    #[error("command `{0}` failed: match failure matches")]
    CommandFailedFailureMatches(String),
    #[error("command `{0}` failed: don't match success matches")]
    CommandFailedSuccsessMatches(String),
    #[error("no engine provided to worker `{0}`")]
    NoEngine(String),
    #[error("failed tsort in {0}")]
    TSort(String),
    #[error("cannot use vars `{0}` from the `{1}`")]
    UseVars(String, PathBuf),
    #[error("failed to setup worker `{0}`")]
    WorkerSetupFailed(String),
}

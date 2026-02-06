use std::io;
use std::path::PathBuf;

use clap::{Command, Parser, Subcommand};
use clap_complete::{Generator, Shell, generate};

use crate::engine::ExistsAction;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print shell completion
    Completion { shell: Shell },

    #[clap(alias = "clean")]
    /// Cleanup all artifacts: workers, networks and storages
    Cleanup {
        #[arg(long, short, default_value = "LM.toml")]
        manifest: PathBuf,
    },

    /// Create lineup manifest
    Init {
        #[arg(long, short, default_value = "default")]
        profile: String,
        #[arg(long, short, default_value = "LM.toml")]
        manifest: PathBuf,
        #[arg(long, short, required = false)]
        extra_vars: Vec<String>,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, infer_long_args(true))]
pub struct Cli {
    #[arg(long, short)]
    pub manifest: Option<PathBuf>,

    #[arg(long, value_name("NUM"))]
    pub num_threads: Option<usize>,

    #[arg(
        long,
        value_name("LEVEL"),
        value_parser(["off", "error", "warn", "info", "debug", "trace"]),
    )]
    pub log_level: Option<String>,

    #[arg(long, value_name("ACTION"))]
    pub worker_exists: Option<ExistsAction>,

    #[arg(
        long,
        alias = "no-clean",
        group = "cleanup-grp",
        required = false,
        help = "Do not cleanup after successefully run all tasks"
    )]
    pub no_cleanup: bool,

    #[arg(
        long,
        alias = "clean",
        group = "cleanup-grp",
        required = false,
        help = "Cleanup after successefully run all tasks"
    )]
    pub cleanup: bool,

    #[arg(long, alias = "clean-before", required = false, help = "Cleanup before running tasks")]
    pub cleanup_before: bool,

    #[arg(long, short, required = false)]
    pub extra_vars: Vec<String>,

    #[arg(
        long,
        required = false,
        num_args = 1,
        value_name = "DIR",
        help = "Store fs vars in the dir insted of a tmpdir"
    )]
    pub fs_var_dir: Option<PathBuf>,

    #[arg(
        long,
        required = false,
        num_args = 1,
        value_name = "FILE",
        help = "Skip tasks from history file (use --completed-tasks-append format)"
    )]
    pub skip_history: Option<PathBuf>,

    #[arg(
        long,
        required = false,
        num_args = 1..,
        value_name = "NAME[=VALUE]",
        help = "Skip runnig every taskline with this name (return json encoded value if set)"
    )]
    pub taskline_skip: Vec<String>,

    #[arg(
        long,
        required = false,
        num_args = 1..,
        value_name = "TASK",
        help = "Do not run this tasks from taskset (split nested tasks on .)"
    )]
    pub taskset_skip: Vec<String>,

    #[arg(
        long,
        required = false,
        num_args = 1,
        value_name = "TASK",
        help = "First task run from taskset (split nested tasks on .)"
    )]
    pub taskset_first: Option<String>,

    #[arg(
        long,
        required = false,
        num_args = 1,
        value_name = "TASK",
        help = "Last task run from taskset (split nested tasks on .)"
    )]
    pub taskset_last: Option<String>,

    #[arg(
        long,
        required = false,
        num_args = 1,
        value_name = "FILE",
        help = "Append all completed tasks to a file"
    )]
    pub completed_tasks_append: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

pub fn print_completions<G: Generator>(g: G, cmd: &mut Command) {
    generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

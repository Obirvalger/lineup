use std::io;
use std::path::PathBuf;

use clap::{Command, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};

use crate::engine::ExistsAction;

#[derive(Debug, Subcommand)]
pub enum Commands {
    Completion {
        shell: Shell,
    },
    Clean {
        #[arg(long, short, default_value = "LM.toml")]
        manifest: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, infer_long_args(true))]
pub struct Cli {
    #[arg(long, short, default_value = "LM.toml")]
    pub manifest: PathBuf,

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

    #[arg(long, group = "clean-grp", required = false)]
    pub no_clean: bool,

    #[arg(long, group = "clean-grp", required = false)]
    pub clean: bool,

    #[arg(long, short, required = false)]
    pub extra_vars: Vec<String>,

    #[arg(long, required = false, num_args = 1.., help = "Don not run this tasks from taskset")]
    pub skip_tasks: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

pub fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

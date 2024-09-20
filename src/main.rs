use anyhow::Result;
use clap::{CommandFactory, Parser};
use cmd_lib::run_cmd;
use env_logger::Env;
use log::error;
use rayon::ThreadPoolBuilder;
use scopeguard::defer;
use serde_json::Value;

use crate::cli::{print_completions, Cli, Commands};
use crate::config::Config;
use crate::error::Error;
use crate::render::Render;
use crate::runner::Runner;
use crate::tmpdir::TMPDIR;
use crate::vars::Vars;

mod cli;
mod cmd;
mod config;
mod engine;
mod error;
mod files;
mod items;
mod manifest;
mod matches;
mod module;
mod render;
mod runner;
mod string_or_int;
mod table;
mod task;
mod task_type;
mod taskline;
mod template;
mod tmpdir;
mod tsort;
mod use_unit;
mod vars;
mod worker;

fn parse_extra_vars(extra_vars: &[String]) -> Result<Vars> {
    let mut vars = Vars::new();
    for var in extra_vars {
        if let Some((name, value)) = var.split_once('=') {
            vars.insert(name.parse()?, Value::String(value.to_string()));
        } else {
            return Err(Error::BadExtraVar(var.to_string()).into());
        }
    }

    vars.render(&tera::Context::new(), "extra vars")
}

fn inner_main() -> Result<()> {
    files::install_main_config()?;
    let config = Config::new()?;
    files::install_modules()?;
    let tmpdir = &TMPDIR;
    defer! {
        // ignore fail in removing tmpdir
        let _ = run_cmd!(rm -rf $tmpdir);
    }
    let args = Cli::parse();
    let level = args.log_level.unwrap_or(config.log_level.to_string());
    env_logger::Builder::from_env(Env::default().default_filter_or(level))
        .format_target(false)
        .format_timestamp(None)
        .init();

    if let Some(command) = args.command {
        match command {
            Commands::Completion { shell } => print_completions(shell, &mut Cli::command()),
        }
    } else {
        let mut thread_pool_builder = ThreadPoolBuilder::new();
        if let Some(num_threads) = args.num_threads {
            thread_pool_builder = thread_pool_builder.num_threads(num_threads);
        }
        let thread_pool = thread_pool_builder.build()?;

        let manifest = &args.manifest;

        thread_pool.install(|| -> Result<()> {
            let extra_vars = parse_extra_vars(&args.extra_vars)?;
            let mut runner = Runner::from_manifest(manifest, &extra_vars.context()?)?;
            runner.set_worker_exists_action(args.worker_exists);
            // Do after initializing to overwrite vars from manifest
            runner.add_extra_vars(extra_vars);
            runner.skip_tasks(&args.skip_tasks);
            runner.run()?;

            if config.clean {
                if !args.no_clean {
                    runner.clean()?;
                }
            } else if args.clean {
                runner.clean()?;
            }

            Ok(())
        })?;
    }

    Ok(())
}

fn main() {
    inner_main().unwrap_or_else(|err| {
        // try to init logger if error occures before logger inited in inner_main
        _ = env_logger::Builder::from_env(Env::default().default_filter_or("error"))
            .format_target(false)
            .format_timestamp(None)
            .try_init();
        error!("{:#}", err);
        std::process::exit(1);
    });
}

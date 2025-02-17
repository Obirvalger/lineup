use anyhow::Error as AnyhowError;
use anyhow::Result;
use clap::{CommandFactory, Parser};
use cmd_lib::run_cmd;
use env_logger::Env;
use log::error;
use rayon::ThreadPoolBuilder;
use scopeguard::defer;
use serde_json::Value;

use crate::cli::{print_completions, Cli, Commands};
use crate::config::{config_initialized, CONFIG};
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
mod exception;
mod files;
mod fs_var;
mod init;
mod items;
mod manifest;
mod matches;
mod module;
mod network;
mod render;
mod runner;
mod storage;
mod string_or_int;
mod table;
mod task;
mod task_result;
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
    config::init()?;
    files::install_all()?;
    let tmpdir = &TMPDIR;
    defer! {
        // ignore fail in removing tmpdir
        let _ = run_cmd!(rm -rf $tmpdir);
    }
    let args = Cli::parse();
    let level = args.log_level.unwrap_or(CONFIG.log_level.to_string());
    env_logger::Builder::from_env(Env::default().default_filter_or(level))
        .format_target(false)
        .format_timestamp(None)
        .init();

    if let Some(command) = args.command {
        match command {
            Commands::Completion { shell } => print_completions(shell, &mut Cli::command()),
            Commands::Clean { manifest } => {
                let mut runner = Runner::from_manifest(manifest, &Default::default())?;
                runner.clean()?;
            }
            Commands::Init { profile, manifest, extra_vars } => {
                let extra_vars = parse_extra_vars(&extra_vars)?;
                init::manifest(profile, &manifest, extra_vars.context()?)?
            }
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

            if CONFIG.clean {
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

fn show_error_indent<K: AsRef<str>, V: AsRef<str>>(key: K, value: V) {
    let key = key.as_ref();
    let value = value.as_ref();
    let lines = value.lines().collect::<Vec<_>>();
    let prefix = format!("  {key}: ");
    let indent = "  ";
    let default_lines_number = if config_initialized() { CONFIG.error.context_lines } else { 10 };
    let lines_number = std::env::var("LINEUP_CONTEXT_LINES")
        .unwrap_or(default_lines_number.to_string())
        .parse::<usize>()
        .unwrap_or(default_lines_number);

    if lines.len() <= 1 {
        error!("{prefix}`{value}`");
    } else {
        error!("{prefix}```");
        for (number, line) in lines.iter().enumerate() {
            if number < lines_number {
                error!("{indent}{line}");
            } else {
                error!("... (show only $LINEUP_CONTEXT_LINES [{lines_number}] lines)");
                break;
            }
        }
        error!("{indent}```");
    }
}

fn show_error(err: AnyhowError) {
    let mut backtrace = vec![];
    let mut contexts = vec![];
    let mut errors = vec![];

    for cause in err.chain() {
        match cause.to_string() {
            msg if msg.starts_with("taskset task: ") => {
                backtrace.push(msg.to_string());
            }
            msg if msg.starts_with("taskline: ") => {
                backtrace.push(msg.to_string());
            }
            msg if msg.starts_with("item: ") => {
                if let Some(last) = backtrace.last_mut() {
                    last.push_str(&format!(", {}", msg));
                }
            }
            msg if msg.starts_with("context_json: ") => {
                contexts.push(msg.trim_start_matches("context_json: ").to_string());
            }
            _ => errors.push(cause.to_string()),
        }
    }

    let error = errors.join(": ");
    let lines = error.lines().collect::<Vec<_>>();
    for (number, line) in lines.iter().enumerate() {
        if number == 0 {
            error!("{line}");
        } else {
            error!("  {line}");
        }
    }

    if config_initialized() && CONFIG.error.context {
        if !contexts.is_empty() {
            error!("context:");
        }
        for context in contexts.iter().rev() {
            let context: Vec<(String, String)> =
                serde_json::from_str(context).expect("Can't deserialize serialized error context");
            for (key, value) in context {
                show_error_indent(key, value);
            }
        }
    }

    if config_initialized() && CONFIG.error.backtrace {
        if !backtrace.is_empty() {
            error!("backtrace:");
        }
        for task in backtrace {
            error!("> {}", task);
        }
    }
}

fn main() {
    inner_main().unwrap_or_else(|err| {
        // try to init logger if error occures before logger inited in inner_main
        _ = env_logger::Builder::from_env(Env::default().default_filter_or("error"))
            .format_target(false)
            .format_timestamp(None)
            .try_init();

        if let Some(Error::User(msg, code)) = &err.downcast_ref::<Error>() {
            if !msg.is_empty() {
                error!("{}", msg);
            }
            std::process::exit(*code);
        }

        show_error(err);
        std::process::exit(1);
    });
}

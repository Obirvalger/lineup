use std::collections::HashMap;
use std::path::PathBuf;

pub use tera::Context;

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use cmd_lib::run_fun;
use inquire::{Confirm, Text};
use lazy_static::lazy_static;
use serde_json::value::{to_value, Value};
use serde_json::{to_string, to_string_pretty};
use tera::Tera;

use crate::cmd::Cmd;
use crate::error::Error;
use crate::fs_var::FsVar;
use crate::tmpdir::TMPDIR;

fn wrap_error(error: anyhow::Error) -> tera::Error {
    tera::Error::msg(error)
}

type FilterAnyhow = Box<dyn Fn(&Value, &HashMap<String, Value>) -> Result<Value> + Sync + Send>;
type FilterTera =
    Box<dyn Fn(&Value, &HashMap<String, Value>) -> tera::Result<Value> + Sync + Send>;

fn wrap_filter(f: FilterAnyhow) -> FilterTera {
    Box::new(move |value, args| f(value, args).map_err(wrap_error))
}

type FunctionAnyhow = Box<dyn Fn(&HashMap<String, Value>) -> Result<Value> + Sync + Send>;
type FunctionTera = Box<dyn Fn(&HashMap<String, Value>) -> tera::Result<Value> + Sync + Send>;

fn wrap_function(f: FunctionAnyhow) -> FunctionTera {
    Box::new(move |args| f(args).map_err(wrap_error))
}

fn basename(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::String(value) => {
            let path = PathBuf::from(&value);
            let new_value = path
                .file_name()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| value.to_string());
            Ok(Value::String(new_value))
        }
        _ => Err(error_not_support.into()),
    }
}

fn cond(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::Bool(condition) => {
            let key = if *condition { "if" } else { "else" };
            let new_value =
                args.get(key).cloned().unwrap_or_else(|| Value::String("".to_string()));
            Ok(new_value)
        }
        _ => Err(error_not_support.into()),
    }
}

fn dirname(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::String(value) => {
            let path = PathBuf::from(&value);
            let new_value = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| value.to_string());
            Ok(Value::String(new_value))
        }
        _ => Err(error_not_support.into()),
    }
}

fn fs_helper(name: &str) -> Result<Value> {
    let fs_var = FsVar::new(name)?;
    if !fs_var.exists() {
        bail!(Error::NoFsVar(name.to_string()));
    }

    fs_var.read()
}

fn fs_filter(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::String(name) => fs_helper(name),
        _ => bail!(error_not_support),
    }
}

fn is_empty(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::Array(a) => Ok(to_value(a.is_empty()).unwrap()),
        Value::Object(m) => Ok(to_value(m.is_empty()).unwrap()),
        Value::String(s) => Ok(to_value(s.is_empty()).unwrap()),
        _ => Err(error_not_support.into()),
    }
}

pub fn json_encode(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let pretty = args.get("pretty").and_then(Value::as_bool).unwrap_or(false);

    if pretty {
        to_string_pretty(&value).map(Value::String).map_err(tera::Error::json)
    } else {
        to_string(&value).map(Value::String).map_err(tera::Error::json)
    }
}

fn quote_string(value: &Value) -> tera::Result<String> {
    let error_not_support = "Value of not supported type";
    let s = match value {
        Value::Bool(_) | Value::Number(_) => value.to_string(),
        Value::String(s) => s.to_string(),
        _ => return Err(error_not_support.into()),
    };

    Ok(run_fun!(printf %q $s)?)
}

fn quote(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let error_not_support = "Value of not supported type";
    match value {
        Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            Ok(Value::String(quote_string(value)?))
        }
        Value::Array(a) => {
            let mut result = vec![];
            for value in a {
                match value {
                    Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                        result.push(quote_string(value)?);
                    }
                    _ => return Err(error_not_support.into()),
                }
            }

            let sep = args.get("sep").and_then(|s| s.as_str()).unwrap_or(" ");
            Ok(Value::String(result.join(sep)))
        }
        _ => Err(error_not_support.into()),
    }
}

fn confirm(args: &HashMap<String, Value>) -> tera::Result<Value> {
    let msg = match args.get("msg") {
        Some(val) => match tera::from_value::<String>(val.to_owned()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `confirm` received msg={} but `msg` can only be a string",
                    val
                )));
            }
        },
        None => {
            return Err(tera::Error::msg("Function `confirm` didn't receive a `msg` argument"))
        }
    };
    let default = match args.get("default") {
        Some(val) => match tera::from_value::<bool>(val.to_owned()) {
            Ok(v) => Some(v),
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `confirm` received default={} but `default` can only be a bool",
                    val
                )));
            }
        },
        None => None,
    };

    let mut confirm = Confirm::new(&msg);
    confirm.default = default;
    let ans = confirm.prompt();

    match ans {
        Ok(ans) => Ok(Value::Bool(ans)),
        Err(err) => Err(tera::Error::msg(err)),
    }
}

fn fs_function(args: &HashMap<String, Value>) -> Result<Value> {
    let error_not_support = "Value of not supported type";
    let name = match args.get("name") {
        Some(val) => val,
        None => bail!("Function `fs` didn't receive a `name` argument"),
    };
    match name {
        Value::String(name) => fs_helper(name),
        _ => bail!(error_not_support),
    }
}

fn input(args: &HashMap<String, Value>) -> Result<Value> {
    let text = if let Some(msg) = args.get("msg") {
        match msg {
            Value::String(msg) => Text::new(msg).prompt()?,
            _ => bail!(Error::WrongArgumentType("msg".to_string())),
        }
    } else {
        bail!(Error::NoArgument("msg".to_string()))
    };

    Ok(Value::String(text))
}

fn host_cmd(args: &HashMap<String, Value>) -> tera::Result<Value> {
    let cmd = match args.get("cmd") {
        Some(val) => val,
        None => {
            return Err(tera::Error::msg("Function `host_cmd` didn't receive a `cmd` argument"))
        }
    };

    let cmd = match cmd {
        Value::String(cmd) => Cmd::from_args_str(["sh", "-c", cmd]),
        Value::Array(a) => {
            let mut cmd = vec![];
            for value in a {
                match value {
                    Value::String(s) => {
                        cmd.push(s);
                    }
                    _ => {
                        return Err(tera::Error::msg(format!(
                            "Function `host_cmd` received cmd array with element={} but `cmd` \
                             can only contain a string elements",
                            value
                        )))
                    }
                }
            }

            Cmd::from_args_str(&cmd)
        }
        _ => {
            return Err(tera::Error::msg(format!(
                "Function `host_cmd` received cmd={} but `cmd` can only be a string or an array",
                cmd
            )))
        }
    };

    let check = match args.get("check") {
        Some(val) => match tera::from_value::<bool>(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `host_cmd` received check={} but `check` can only be a boolean",
                    val
                )));
            }
        },
        None => true,
    };

    let capture_stdout = match args.get("capture") {
        Some(val) => match tera::from_value::<String>(val.clone()) {
            Ok(v) => match v.as_str() {
                "stdout" => true,
                "stderr" => false,
                _ => {
                    return Err(tera::Error::msg(format!(
                        "Function `host_cmd` received capture={} but `capture` can only be \
                         `stdout` or `stderr`",
                        val
                    )));
                }
            },
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `host_cmd` received capture={} but `capture` can only be a string",
                    val
                )));
            }
        },
        None => true,
    };

    let args = cmd.get_args();
    let out = cmd.run().map_err(tera::Error::msg)?;

    if check && !out.success() {
        return Err(tera::Error::msg(format!("In function `host_cmd` command `{}` failed", args)));
    }

    if capture_stdout {
        Ok(Value::String(out.stdout().trim_end().to_string()))
    } else {
        Ok(Value::String(out.stderr().trim_end().to_string()))
    }
}

fn tmpdir(_args: &HashMap<String, Value>) -> tera::Result<Value> {
    Ok(Value::String(TMPDIR.display().to_string()))
}

pub fn render<S: ToString, P: AsRef<str>>(
    context: &Context,
    template: S,
    place: P,
) -> Result<String> {
    lazy_static! {
        pub static ref RENDERER: Tera = {
            let mut tera = Tera::default();

            tera.register_filter("basename", basename);
            tera.register_filter("cond", cond);
            tera.register_filter("dirname", dirname);
            tera.register_filter("fs", wrap_filter(Box::new(fs_filter)));
            tera.register_filter("is_empty", is_empty);
            tera.register_filter("j", json_encode);
            tera.register_filter("json", json_encode);
            tera.register_filter("q", quote);
            tera.register_filter("quote", quote);

            tera.register_function("confirm", confirm);
            tera.register_function("fs", wrap_function(Box::new(fs_function)));
            tera.register_function("input", wrap_function(Box::new(input)));
            tera.register_function("host_cmd", host_cmd);
            tera.register_function("tmpdir", tmpdir);
            tera
        };
    }
    RENDERER
        .to_owned()
        .render_str(&template.to_string(), context)
        .with_context(|| format!("Failed to render template in {}", place.as_ref()))
}

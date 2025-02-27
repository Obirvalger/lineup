use std::process::Command;

use anyhow::Result;

pub fn quote<S: AsRef<str>>(s: S) -> Result<String> {
    let mut cmd = Command::new("printf");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg("%q");
    cmd.arg(s.as_ref());
    let out = cmd.spawn()?.wait_with_output()?.stdout;
    let quoted = String::from_utf8(out)?;

    Ok(quoted)
}

pub fn quote_args<S: AsRef<str>>(args: &[S]) -> Result<String> {
    let mut cmd = Vec::with_capacity(args.len());
    for arg in args {
        let quoted = quote(arg)?;
        cmd.push(quoted);
    }
    let command = cmd.join(" ");

    Ok(command)
}

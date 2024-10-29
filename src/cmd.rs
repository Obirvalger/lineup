use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output};

use anyhow::Result;

use crate::error::Error;

#[derive(Debug)]
pub struct Cmd {
    inner: Command,
    stdin: Option<String>,
}

impl Cmd {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self { inner: Command::new(program), stdin: None }
    }

    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut iter = args.into_iter();
        let mut cmd = Command::new(iter.next().expect("Run Cmd::from args on empty sequence"));
        for arg in iter {
            cmd.arg(arg);
        }

        Self { inner: cmd, stdin: None }
    }

    pub fn from_args_str<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = args.into_iter();
        let mut cmd =
            Command::new(iter.next().expect("Run Cmd::from args on empty sequence").as_ref());
        for arg in iter {
            cmd.arg(arg.as_ref());
        }

        Self { inner: cmd, stdin: None }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn get_args(&self) -> String {
        let mut args =
            vec![format!("\"{}\"", self.inner.get_program().to_string_lossy().to_string())];
        args.extend(self.inner.get_args().map(|a| format!("\"{}\"", a.to_string_lossy())));

        args.join(" ")
    }

    pub fn set_stdin<S: AsRef<str>>(&mut self, stdin: S) -> &mut Self {
        self.stdin = Some(stdin.as_ref().to_string());
        self
    }

    pub fn run(mut self) -> Result<CmdOut> {
        self.inner.stdin(std::process::Stdio::piped());
        self.inner.stdout(std::process::Stdio::piped());
        self.inner.stderr(std::process::Stdio::piped());

        let mut child = self.inner.spawn()?;
        if let Some(stdin) = self.stdin {
            child.stdin.as_mut().ok_or(Error::ChildStdin)?.write_all(stdin.as_bytes())?;
        }

        Ok(CmdOut::new(child.wait_with_output()?))
    }
}

#[derive(Clone, Debug)]
pub struct CmdOut {
    inner: Output,
    success_codes: Vec<i32>,
}

impl CmdOut {
    pub fn new(output: Output) -> Self {
        Self { inner: output, success_codes: vec![0] }
    }

    pub fn success_codes(&mut self, success_codes: &[i32]) {
        self.success_codes = Vec::from(success_codes);
    }

    pub fn success(&self) -> bool {
        if self.success_codes.is_empty() {
            return true;
        }

        if let Some(code) = self.inner.status.code() {
            self.success_codes.contains(&code)
        } else {
            false
        }
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.inner.stdout).to_string()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.inner.stderr).to_string()
    }
}

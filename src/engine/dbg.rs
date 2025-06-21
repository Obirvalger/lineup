use std::path::Path;

use anyhow::Result;

use crate::cmd::Cmd;
use crate::engine::EngineBase;

#[derive(Clone, Debug)]
pub struct EngineDbg {
    pub base: EngineBase,
}

impl EngineDbg {
    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        println!(
            "Worker {}: upload(file) file from {} to {}:{}",
            name.as_ref(),
            src.as_ref().display(),
            name.as_ref(),
            dst.as_ref().display()
        );

        Ok(())
    }

    pub fn get<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        println!(
            "Worker {}: download(get) file from {}:{} to {}",
            name.as_ref(),
            name.as_ref(),
            src.as_ref().display(),
            dst.as_ref().display()
        );

        Ok(())
    }

    pub fn exec_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, args: &[S]) -> Cmd {
        let args = args.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
        println!("Worker {}: exec {:?}", name.as_ref(), args);

        Cmd::new("true")
    }

    pub fn start<N: AsRef<str>>(&self, name: N) -> Result<()> {
        println!("Worker {}: start", name.as_ref());

        Ok(())
    }

    pub fn restart<N: AsRef<str>>(&self, name: N) -> Result<()> {
        println!("Worker {}: restart", name.as_ref());

        Ok(())
    }

    pub fn stop<N: AsRef<str>>(&self, name: N) -> Result<()> {
        println!("Worker {}: stop", name.as_ref());

        Ok(())
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, name: N, command: S) -> Cmd {
        println!("Worker {}: run shell command `{}`", name.as_ref(), command.as_ref());

        Cmd::new("true")
    }
}

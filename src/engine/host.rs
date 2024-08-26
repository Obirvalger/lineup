use std::path::Path;

use anyhow::Result;
use cmd_lib::run_cmd;

use crate::cmd::Cmd;
use crate::engine::EngineBase;

#[derive(Clone, Debug)]
pub struct EngineHost {
    pub base: EngineBase,
}

impl EngineHost {
    pub fn copy<N: AsRef<str>, S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        _name: N,
        src: S,
        dst: D,
    ) -> Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        run_cmd!(cp $src $dst)?;

        Ok(())
    }

    pub fn exec_cmd<N: AsRef<str>, S: AsRef<str>>(&self, _name: N, args: &[S]) -> Cmd {
        Cmd::from_args_str(args)
    }

    pub fn shell_cmd<N: AsRef<str>, S: AsRef<str>>(&self, _name: N, command: S) -> Cmd {
        let mut cmd = Cmd::new("sh");
        cmd.arg("-c");
        cmd.arg(command.as_ref());

        cmd
    }
}

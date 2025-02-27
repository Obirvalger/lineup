use std::path::PathBuf;

use cmd_lib::{run_cmd, run_fun};
use once_cell::sync::Lazy;

pub static TMPDIR: Lazy<PathBuf> = Lazy::new(|| {
    let tmpdir =
        PathBuf::from(run_fun! {mktemp -dt lineup.XXXXXXXX}.expect("can't create tmpdir"));
    run_cmd! {mkdir $tmpdir/tmpfiles }.expect("can't create tmpdir/tmpfiles");
    tmpdir
});

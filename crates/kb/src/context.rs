use std::env;
use std::path::PathBuf;

use crate::cli::GlobalArgs;

#[derive(Debug)]
pub struct RuntimeContext {
    pub json: bool,
    pub cwd: PathBuf,
}

impl RuntimeContext {
    pub fn from_global_args(global: &GlobalArgs) -> Self {
        Self {
            json: global.json,
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

use crate::manifest::TasklineElem;

#[derive(Clone, Debug, Serialize)]
pub enum Taskline {
    File { file: PathBuf, name: String },
    Line(Vec<TasklineElem>),
}

impl Taskline {
    pub fn as_line(&self) -> Option<&Vec<TasklineElem>> {
        if let Self::Line(line) = self {
            Some(line)
        } else {
            None
        }
    }

    pub fn is_line(&self) -> bool {
        matches!(self, Self::Line(_))
    }
}

impl Default for Taskline {
    fn default() -> Self {
        Self::Line(Default::default())
    }
}

impl<'de> Deserialize<'de> for Taskline {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        type Line = Vec<TasklineElem>;
        let line = Line::deserialize(deserializer)?;
        Ok(Taskline::Line(line))
    }
}

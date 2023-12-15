use std::fmt;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Number(i64),
}

impl fmt::Display for StringOrInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
        }
    }
}

impl Render for StringOrInt {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let s = self.to_string().render(context, place)?;
        Ok(StringOrInt::String(s))
    }
}

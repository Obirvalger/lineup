use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;

use crate::template::{render, Context};

pub trait Render: Sized {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self>;
}

pub trait RenderMut: Sized {
    fn render_mut<S: AsRef<str>>(&mut self, context: &Context, place: S) -> Result<()>;
}

impl Render for String {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        render(context, self, place)
    }
}

impl Render for PathBuf {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        Ok(PathBuf::from(self.display().to_string().render(context, place)?))
    }
}

impl<R: Render> Render for Option<R> {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        self.as_ref().map(|r| r.render(context, place)).transpose()
    }
}

impl<R: Render> Render for Vec<R> {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        self.iter().map(|r| r.render(context, place.as_ref())).collect()
    }
}

impl RenderMut for Value {
    fn render_mut<S: AsRef<str>>(&mut self, context: &Context, place: S) -> Result<()> {
        match self {
            Value::String(s) => *s = s.render(context, place.as_ref())?,
            Value::Object(m) => {
                for (_, v) in m.iter_mut() {
                    v.render_mut(context, place.as_ref())?;
                }
            }
            Value::Array(a) => {
                for v in a.iter_mut() {
                    v.render_mut(context, place.as_ref())?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl Render for Value {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let mut value = self.to_owned();
        value.render_mut(context, place)?;

        Ok(value)
    }
}

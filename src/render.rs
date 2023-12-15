use std::path::PathBuf;

use anyhow::Result;

use crate::template::{render, Context};

pub trait Render: Sized {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self>;
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

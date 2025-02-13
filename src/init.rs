use std::fs;
use std::path::Path;

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};

use crate::config::CONFIG;
use crate::error::Error;
use crate::render::Render;
use crate::template::Context;

pub fn manifest<S: AsRef<str>>(profile: S, manifest_path: &Path, context: Context) -> Result<()> {
    let profile_name = profile.as_ref();
    let profile = &CONFIG
        .init
        .profiles
        .get(profile_name)
        .ok_or_else(|| Error::BadInitProfile(profile_name.to_string()))?;

    if manifest_path != Path::new("-") && manifest_path.exists() {
        bail!(Error::InitManifestExists(manifest_path.to_owned()))
    } else {
        let manifest_str = if profile.render {
            let mut new_context = profile.vars.context()?;
            new_context.extend(context);
            profile.manifest.render(&new_context, "manifest in init profile in config")?
        } else {
            profile.manifest.to_string()
        };

        if manifest_path != Path::new("-") {
            fs::write(manifest_path, manifest_str).with_context(|| {
                format!("Failed to initialize manifest `{}`", manifest_path.display())
            })?;
        } else {
            print!("{manifest_str}");
        }
    }

    Ok(())
}

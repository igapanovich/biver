use crate::configuration::Configuration;
use crate::error::Result;
use crate::external_command;
use std::path::Path;

pub fn create_patch(config: &Configuration, old: &Path, new: &Path, patch: &Path) -> Result<()> {
    let args = [
        ("old", &*old.to_string_lossy()),
        ("new", &*new.to_string_lossy()),
        ("patch", &*patch.to_string_lossy()),
    ];
    external_command::run_templated_command(&config.create_patch_command, &args)?;
    Ok(())
}

pub fn apply_patch(config: &Configuration, old: &Path, patch: &Path, new: &Path) -> Result<()> {
    let args = [
        ("old", &*old.to_string_lossy()),
        ("patch", &*patch.to_string_lossy()),
        ("new", &*new.to_string_lossy()),
    ];
    external_command::run_templated_command(&config.apply_patch_command, &args)?;
    Ok(())
}

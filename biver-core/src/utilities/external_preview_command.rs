use crate::ExternalCommand;
use crate::error::Result;
use crate::utilities::external_command;
use std::path::Path;

pub fn create_preview(command: &ExternalCommand, source: &Path, destination: &Path) -> Result<()> {
    let args = &[
        ("in", &*source.to_string_lossy()),
        ("out", &*destination.to_string_lossy()),
    ];
    external_command::run_templated_command(command, args)?;
    Ok(())
}

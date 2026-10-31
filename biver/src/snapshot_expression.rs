use crate::error::{Error, Result, Severity};
use biver_core::data::Snapshot;

pub fn resolve_optional(expr: Option<&str>) -> Result<Snapshot> {
    let Some(expr) = expr else {
        return Ok(Snapshot::Main);
    };

    resolve(expr)
}

pub fn resolve(expr: &str) -> Result<Snapshot> {
    if expr.eq_ignore_ascii_case("main") {
        return Ok(Snapshot::Main);
    }

    match expr {
        "1" => Ok(Snapshot::Snapshot1),
        "2" => Ok(Snapshot::Snapshot2),
        "3" => Ok(Snapshot::Snapshot3),
        "4" => Ok(Snapshot::Snapshot4),
        "5" => Ok(Snapshot::Snapshot5),
        "6" => Ok(Snapshot::Snapshot6),
        "7" => Ok(Snapshot::Snapshot7),
        "8" => Ok(Snapshot::Snapshot8),
        _ => Err(Error {
            message: format!("Invalid snapshot identifier '{}'", expr),
            severity: Severity::Error,
        }),
    }
}

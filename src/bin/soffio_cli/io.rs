#![deny(clippy::all, clippy::pedantic)]

use std::fs;
use std::path::PathBuf;

use crate::client::CliError;

pub fn read_value(val: Option<String>, file: Option<PathBuf>) -> Result<String, CliError> {
    if let Some(path) = file {
        let data = fs::read_to_string(&path).map_err(|source| CliError::InputFile {
            path: path.display().to_string(),
            source,
        })?;
        Ok(data)
    } else if let Some(v) = val {
        Ok(v)
    } else {
        Err(CliError::InvalidInput("value required".into()))
    }
}

pub fn read_opt_value(
    val: Option<String>,
    file: Option<PathBuf>,
) -> Result<Option<String>, CliError> {
    if let Some(path) = file {
        let data = fs::read_to_string(&path).map_err(|source| CliError::InputFile {
            path: path.display().to_string(),
            source,
        })?;
        return Ok(Some(data));
    }
    Ok(val)
}

pub fn parse_time_opt(val: Option<String>) -> Result<Option<time::OffsetDateTime>, CliError> {
    if let Some(v) = val {
        let parsed =
            time::OffsetDateTime::parse(&v, &time::format_description::well_known::Rfc3339)
                .map_err(|e| CliError::InvalidInput(e.to_string()))?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

pub fn to_value<T: serde::Serialize>(value: T) -> Result<serde_json::Value, CliError> {
    serde_json::to_value(value).map_err(|e| CliError::InvalidInput(e.to_string()))
}

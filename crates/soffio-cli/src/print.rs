#![deny(clippy::all, clippy::pedantic)]

use crate::client::CliError;
use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let out = serde_json::to_string_pretty(value)
        .map_err(|e| CliError::Server(format!("failed to render output: {e}")))?;
    println!("{out}");
    Ok(())
}

pub fn json_value(value: &serde_json::Value) -> Result<(), CliError> {
    print_json(value)
}

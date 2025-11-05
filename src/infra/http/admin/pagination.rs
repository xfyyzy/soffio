use axum::http::StatusCode;

use crate::application::error::HttpError;

pub(crate) const CURSOR_ROOT_TOKEN: &str = "~";

pub(crate) fn parse_cursor_history(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

pub(crate) fn encode_cursor_token(cursor: Option<&str>) -> String {
    match cursor.filter(|value| !value.is_empty()) {
        Some(value) => value.to_string(),
        None => CURSOR_ROOT_TOKEN.to_string(),
    }
}

pub(crate) fn decode_cursor_token(token: &str) -> Option<String> {
    if token == CURSOR_ROOT_TOKEN {
        None
    } else {
        Some(token.to_string())
    }
}

pub(crate) fn join_cursor_history(tokens: &[String]) -> Option<String> {
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join("."))
    }
}

pub(crate) fn decode_cursor_param<T, F, E>(
    raw: Option<&str>,
    decoder: F,
    source: &'static str,
) -> Result<Option<T>, HttpError>
where
    F: Fn(&str) -> Result<T, E>,
    E: std::fmt::Display,
{
    match raw {
        Some(value) if !value.is_empty() => decoder(value).map(Some).map_err(|err| {
            HttpError::new(
                source,
                StatusCode::BAD_REQUEST,
                "Invalid cursor",
                err.to_string(),
            )
        }),
        _ => Ok(None),
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CursorState {
    history: Vec<String>,
    current: Option<String>,
}

impl CursorState {
    pub(crate) fn new(current: Option<String>, trail: Option<String>) -> Self {
        Self {
            history: parse_cursor_history(trail.as_deref()),
            current,
        }
    }

    pub(crate) fn history_tokens(&self) -> &[String] {
        &self.history
    }

    pub(crate) fn clone_history(&self) -> Vec<String> {
        self.history.clone()
    }

    pub(crate) fn current_token(&self) -> Option<String> {
        self.current.clone()
    }

    pub(crate) fn current_token_ref(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub(crate) fn decode_with<T, F, E>(
        &self,
        decoder: F,
        source: &'static str,
    ) -> Result<Option<T>, HttpError>
    where
        F: Fn(&str) -> Result<T, E>,
        E: std::fmt::Display,
    {
        decode_cursor_param(self.current.as_deref(), decoder, source)
    }
}

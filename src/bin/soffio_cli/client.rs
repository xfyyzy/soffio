#![deny(clippy::all, clippy::pedantic)]

use axum::http::HeaderValue;
use reqwest::{Client, Method, Response, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::args::Cli;
use std::fs;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("site URL is required (use --site or SOFFIO_SITE_URL)")]
    MissingSite,
    #[error("api key is required (use --key-file or SOFFIO_API_KEY)")]
    MissingKey,
    #[error("failed to read key file: {0}")]
    KeyFile(std::io::Error),
    #[error("failed to read input file {path}: {source}")]
    InputFile {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server error: {0}")]
    Server(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Clone, Debug)]
pub struct Ctx {
    pub client: Client,
    pub base: Url,
    pub key: String,
}

impl Ctx {
    pub fn new(site: &str, key: String) -> Result<Self, CliError> {
        let base = Url::parse(site)?.join("/")?;
        let client = Client::builder().user_agent(Self::user_agent()).build()?;
        Ok(Self { client, base, key })
    }

    pub fn user_agent() -> &'static str {
        concat!("soffio-cli/", env!("CARGO_PKG_VERSION"))
    }

    pub fn auth_header(&self) -> Result<HeaderValue, CliError> {
        HeaderValue::from_str(&format!("Bearer {}", self.key))
            .map_err(|e| CliError::InvalidInput(e.to_string()))
    }

    pub fn url(&self, path: &str) -> Result<Url, CliError> {
        self.base.join(path).map_err(CliError::Url)
    }

    pub async fn request<T: for<'de> Deserialize<'de> + Serialize + std::fmt::Debug>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
    ) -> Result<T, CliError> {
        let mut url = self.url(path)?;
        if let Some(q) = query {
            url.set_query(None);
            let mut qp = url.query_pairs_mut();
            for (k, v) in q {
                qp.append_pair(k, v);
            }
        }

        let mut req = self
            .client
            .request(method, url)
            .header(axum::http::header::AUTHORIZATION, self.auth_header()?);
        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await?;
        Self::handle(resp).await
    }

    pub async fn request_unit(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
    ) -> Result<(), CliError> {
        let mut url = self.url(path)?;
        if let Some(q) = query {
            url.set_query(None);
            let mut qp = url.query_pairs_mut();
            for (k, v) in q {
                qp.append_pair(k, v);
            }
        }

        let mut req = self
            .client
            .request(method, url)
            .header(axum::http::header::AUTHORIZATION, self.auth_header()?);
        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(CliError::Server(format!("status {status} body {text}")));
        }
        Ok(())
    }

    async fn handle<T: for<'de> Deserialize<'de>>(resp: Response) -> Result<T, CliError> {
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            return Err(CliError::Server(format!("status {status} body {text}")));
        }
        let val = serde_json::from_slice(&bytes)
            .map_err(|e| CliError::Server(format!("failed to parse body: {e}")))?;
        Ok(val)
    }

    pub async fn request_no_body(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, String)]>,
    ) -> Result<(), CliError> {
        let mut url = self.url(path)?;
        if let Some(q) = query {
            url.set_query(None);
            let mut qp = url.query_pairs_mut();
            for (k, v) in q {
                qp.append_pair(k, v);
            }
        }

        let resp = self
            .client
            .request(method, url)
            .header(axum::http::header::AUTHORIZATION, self.auth_header()?)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(CliError::Server(format!("status {status} body {text}")));
        }
        Ok(())
    }
}

pub fn build_ctx_from_cli(cli: &Cli) -> Result<Ctx, CliError> {
    let site = cli.site.clone().ok_or(CliError::MissingSite)?;
    let key = if let Some(path) = &cli.key_file {
        fs::read_to_string(path)
            .map_err(CliError::KeyFile)?
            .trim()
            .to_string()
    } else {
        cli.api_key_env.clone().ok_or(CliError::MissingKey)?
    };

    Ctx::new(&site, key)
}

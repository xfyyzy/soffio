use std::fmt;

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PostStatusArg {
    Draft,
    Published,
    Archived,
    Error,
}

impl PostStatusArg {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for PostStatusArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PageStatusArg {
    Draft,
    Published,
    Archived,
    Error,
}

impl PageStatusArg {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for PageStatusArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum NavDestArg {
    Internal,
    External,
}

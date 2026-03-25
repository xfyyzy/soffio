use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub action: SettingsCmd,
}

#[derive(Subcommand, Debug)]
pub enum SettingsCmd {
    /// Show settings
    Get,
    /// Patch settings (only provided fields)
    Patch(Box<SettingsPatchArgs>),
}

#[derive(Parser, Debug)]
pub struct SettingsPatchArgs {
    #[arg(long)]
    pub brand_title: Option<String>,
    #[arg(long)]
    pub brand_href: Option<String>,
    #[arg(long)]
    pub footer_copy: Option<String>,
    #[arg(long)]
    pub homepage_size: Option<i32>,
    #[arg(long)]
    pub admin_page_size: Option<i32>,
    #[arg(long)]
    pub show_tag_aggregations: Option<bool>,
    #[arg(long)]
    pub show_month_aggregations: Option<bool>,
    #[arg(long)]
    pub tag_filter_limit: Option<i32>,
    #[arg(long)]
    pub month_filter_limit: Option<i32>,
    #[arg(long)]
    pub timezone: Option<String>,
    #[arg(long)]
    pub meta_title: Option<String>,
    #[arg(long)]
    pub meta_description: Option<String>,
    #[arg(long)]
    pub og_title: Option<String>,
    #[arg(long)]
    pub og_description: Option<String>,
    #[arg(long)]
    pub public_site_url: Option<String>,
    #[arg(long)]
    pub global_toc_enabled: Option<bool>,
    #[arg(long)]
    pub favicon_svg: Option<String>,
    #[arg(long)]
    pub favicon_svg_file: Option<PathBuf>,
}

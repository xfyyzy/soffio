use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct JobsArgs {
    #[command(subcommand)]
    pub action: JobsCmd,
}

#[derive(Subcommand, Debug)]
pub enum JobsCmd {
    /// List background jobs
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        job_type: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
}

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "usectl")]
#[command(about = "A simple Linux USE methodology diagnostics CLI")]

pub struct Cli {
    #[arg(long)]
    pub json: bool,

    #[arg(long, value_name = "SECONDS")]
    pub watch: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Show CPU utilization and load information.
    Cpu,

    /// Show memory utilization, swap, faults, and PSI.
    Memory,

    /// Show disk activity counters and IO PSI.
    Disk,

    /// Show network interface counters.
    Network,

    /// Show a combined USE-style system report.
    Analyze,
}

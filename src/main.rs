mod cli;
mod cpu;
mod disk;
mod memory;
mod network;
mod pressure;
mod report;
mod utils;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Cpu => cpu::print_cpu()?,
        Commands::Memory => memory::print_memory()?,
        Commands::Disk => disk::print_disk()?,
        Commands::Network => network::print_network()?,
        Commands::Analyze => report::print_report()?,
    }

    Ok(())
}

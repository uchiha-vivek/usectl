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
use serde::Serialize;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Cpu => {
            if cli.json {
                print_json(&cpu::read_cpu_stats()?)?;
            } else {
                cpu::print_cpu()?;
            }
        }
        Commands::Memory => {
            if cli.json {
                print_json(&memory::read_memory_stats()?)?;
            } else {
                memory::print_memory()?;
            }
        }
        Commands::Disk => {
            if cli.json {
                print_json(&disk::read_disk_stats()?)?;
            } else {
                disk::print_disk()?;
            }
        }
        Commands::Network => {
            if cli.json {
                print_json(&network::read_network_stats()?)?;
            } else {
                network::print_network()?;
            }
        }
        Commands::Analyze => {
            if cli.json {
                let report = serde_json::json!({
                    "cpu": cpu::read_cpu_stats()?,
                    "memory": memory::read_memory_stats()?,
                    "disk": disk::read_disk_stats()?,
                    "network": network::read_network_stats()?,
                });

                print_json(&report)?;
            } else {
                report::print_report()?;
            }
        }
    }

    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

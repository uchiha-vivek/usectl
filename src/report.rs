use anyhow::Result;

use crate::cpu;
use crate::disk;
use crate::memory;
use crate::network;
use crate::utils;

pub fn print_report() -> Result<()> {
    utils::ensure_supported()?;

    let cpu = cpu::read_cpu_stats()?;
    let memory = memory::read_memory_stats()?;
    let disk = disk::read_disk_stats()?;
    let network = network::read_network_stats()?;

    println!("USECTL SYSTEM REPORT");
    println!("====================");
    println!();

    println!("CPU");
    println!(
        "Utilization: {}",
        utils::format_percent(cpu.utilization_percent)
    );
    println!("Saturation:");
    println!("  Load Average: {:.2}", cpu.load_1);
    println!("Errors:");
    println!("  Not available");
    print_cpu_warnings(cpu.utilization_percent, cpu.load_1);
    println!();

    println!("Memory");
    println!(
        "Utilization: {}",
        utils::format_percent(memory.utilization_percent)
    );
    println!("Saturation:");
    match memory.pressure.as_ref().and_then(|p| p.some_avg10) {
        Some(avg10) => println!("  Memory PSI avg10: {avg10:.2}%"),
        None => println!("  Not available"),
    }
    println!("Errors:");
    println!("  Major faults: {}", memory.major_page_faults);
    print_memory_warnings(
        memory.utilization_percent,
        memory.swap_used_kb,
        memory.pressure.as_ref().and_then(|p| p.some_avg10),
    );
    println!();

    println!("Disk");
    println!("Utilization:");
    if disk.devices.is_empty() {
        println!("  Not available");
    } else {
        println!("  Basic disk activity available");
    }
    println!("Saturation:");
    match disk.pressure.as_ref().and_then(|p| p.some_avg10) {
        Some(avg10) => println!("  IO PSI avg10: {avg10:.2}%"),
        None => println!("  Not available"),
    }
    print_disk_warnings(disk.pressure.as_ref().and_then(|p| p.some_avg10));
    println!("Errors:");
    println!("  Not available from current source");
    println!();

    let rx_bytes: u64 = network.iter().map(|iface| iface.rx_bytes).sum();
    let tx_bytes: u64 = network.iter().map(|iface| iface.tx_bytes).sum();
    let rx_drops: u64 = network.iter().map(|iface| iface.rx_drops).sum();
    let tx_drops: u64 = network.iter().map(|iface| iface.tx_drops).sum();
    let rx_errors: u64 = network.iter().map(|iface| iface.rx_errors).sum();
    let tx_errors: u64 = network.iter().map(|iface| iface.tx_errors).sum();

    println!("Network");
    println!("Utilization:");
    if network.is_empty() {
        println!("  Not available");
    } else {
        println!("  RX: {}", utils::format_bytes(rx_bytes as f64));
        println!("  TX: {}", utils::format_bytes(tx_bytes as f64));
    }
    println!("Saturation:");
    println!("  RX Drops: {rx_drops}");
    println!("  TX Drops: {tx_drops}");
    println!("Errors:");
    println!("  RX Errors: {rx_errors}");
    println!("  TX Errors: {tx_errors}");
    print_network_warnings(rx_drops, tx_drops, rx_errors, tx_errors);

    Ok(())
}

fn print_cpu_warnings(utilization_percent: f64, load_1: f64) {
    if utilization_percent > 90.0 {
        println!();
        println!("HIGH CPU UTILIZATION");
        println!("CPU utilization > 90%");
    }

    if load_1 > 8.0 {
        println!();
        println!("HIGH CPU LOAD");
        println!("1-minute load average > 8.0");
    }
}

fn print_memory_warnings(
    utilization_percent: f64,
    swap_used_kb: u64,
    memory_psi_some_avg10: Option<f64>,
) {
    if utilization_percent > 85.0 {
        println!();
        println!("HIGH MEMORY UTILIZATION");
        println!("Memory utilization > 85%");
    }

    if swap_used_kb > 0 {
        println!();
        println!("SWAP IN USE");
        println!("Swap used > 0");
    }

    if memory_psi_some_avg10.is_some_and(|value| value > 5.0) {
        println!();
        println!("MEMORY PRESSURE DETECTED");
        println!("Memory PSI some avg10 > 5%");
    }
}

fn print_disk_warnings(io_psi_some_avg10: Option<f64>) {
    if io_psi_some_avg10.is_some_and(|value| value > 5.0) {
        println!();
        println!("DISK PRESSURE DETECTED");
        println!("IO PSI some avg10 > 5%");
    }
}

fn print_network_warnings(rx_drops: u64, tx_drops: u64, rx_errors: u64, tx_errors: u64) {
    if rx_drops > 0 || tx_drops > 0 {
        println!();
        println!("NETWORK PACKET DROPS DETECTED");
        println!("RX drops or TX drops > 0");
    }

    if rx_errors > 0 || tx_errors > 0 {
        println!();
        println!("NETWORK ERRORS DETECTED");
        println!("RX errors or TX errors > 0");
    }
}

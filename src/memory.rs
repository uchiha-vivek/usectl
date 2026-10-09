use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashMap;

use crate::pressure::{self, PressureStats};
use crate::utils;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
    pub utilization_percent: f64,
    pub swap_total_kb: u64,
    pub swap_used_kb: u64,
    pub page_faults: u64,
    pub major_page_faults: u64,
    pub pressure: Option<PressureStats>,
}

pub fn print_memory() -> Result<()> {
    let stats = read_memory_stats()?;

    println!("Memory");
    println!("Platform : {} ", utils::platform_name());
    println!("Total: {}", utils::format_bytes_from_kb(stats.total_kb));
    println!("Used: {}", utils::format_bytes_from_kb(stats.used_kb));
    println!(
        "Available: {}",
        utils::format_bytes_from_kb(stats.available_kb)
    );
    println!(
        "Utilization: {}",
        utils::format_percent(stats.utilization_percent)
    );
    println!();
    println!(
        "Swap Total: {}",
        utils::format_bytes_from_kb(stats.swap_total_kb)
    );
    println!(
        "Swap Used: {}",
        utils::format_bytes_from_kb(stats.swap_used_kb)
    );
    println!();
    println!("Page Faults: {}", stats.page_faults);
    println!("Major Page Faults: {}", stats.major_page_faults);
    println!();
    pressure::print_pressure("Memory PSI", &stats.pressure);

    Ok(())
}

pub fn read_memory_stats() -> Result<MemoryStats> {
    utils::ensure_supported()?;

    if cfg!(target_os = "macos") {
        return read_macos_memory_stats();
    }

    let mut stats = parse_meminfo(&utils::read_file("/proc/meminfo")?)?;
    let (page_faults, major_page_faults) = parse_vmstat(&utils::read_file("/proc/vmstat")?)?;

    stats.page_faults = page_faults;
    stats.major_page_faults = major_page_faults;
    stats.pressure = pressure::read_pressure("/proc/pressure/memory")?;

    Ok(stats)
}

fn read_macos_memory_stats() -> Result<MemoryStats> {
    let total_bytes = utils::command_output("sysctl", &["-n", "hw.memsize"])?
        .trim()
        .parse::<u64>()?;
    let vm_stat = utils::command_output("vm_stat", &[])?;
    let swap = utils::command_output("sysctl", &["-n", "vm.swapusage"]).unwrap_or_default();

    let page_size = parse_macos_page_size(&vm_stat).unwrap_or(4096);
    let values = parse_macos_vm_stat(&vm_stat);

    let free_pages = values.get("Pages free").copied().unwrap_or(0);
    let inactive_pages = values.get("Pages inactive").copied().unwrap_or(0);
    let speculative_pages = values.get("Pages speculative").copied().unwrap_or(0);
    let available_bytes = (free_pages + inactive_pages + speculative_pages) * page_size;
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let utilization_percent = if total_bytes == 0 {
        0.0
    } else {
        used_bytes as f64 / total_bytes as f64 * 100.0
    };

    let (swap_total_kb, swap_used_kb) = parse_macos_swapusage(&swap).unwrap_or((0, 0));

    Ok(MemoryStats {
        total_kb: total_bytes / 1024,
        available_kb: available_bytes / 1024,
        used_kb: used_bytes / 1024,
        utilization_percent,
        swap_total_kb,
        swap_used_kb,
        page_faults: values.get("Translation faults").copied().unwrap_or(0),
        major_page_faults: values.get("Pageins").copied().unwrap_or(0),
        pressure: None,
    })
}

pub fn parse_meminfo(contents: &str) -> Result<MemoryStats> {
    let values = parse_key_value_kb(contents);

    let total_kb = get_required(&values, "MemTotal")?;
    let available_kb = get_required(&values, "MemAvailable")?;
    let swap_total_kb = *values.get("SwapTotal").unwrap_or(&0);
    let swap_free_kb = *values.get("SwapFree").unwrap_or(&0);

    // MemAvailable estimates memory available for new applications without
    // swapping, so reclaimable cache is not counted as unavailable memory.
    let used_kb = total_kb.saturating_sub(available_kb);
    let utilization_percent = if total_kb == 0 {
        0.0
    } else {
        used_kb as f64 / total_kb as f64 * 100.0
    };

    Ok(MemoryStats {
        total_kb,
        available_kb,
        used_kb,
        utilization_percent,
        swap_total_kb,
        swap_used_kb: swap_total_kb.saturating_sub(swap_free_kb),
        page_faults: 0,
        major_page_faults: 0,
        pressure: None,
    })
}

pub fn parse_vmstat(contents: &str) -> Result<(u64, u64)> {
    let mut page_faults = None;
    let mut major_page_faults = None;

    for line in contents.lines() {
        let mut fields = line.split_whitespace();

        let Some(key) = fields.next() else {
            continue;
        };

        let Some(value) = fields.next() else {
            continue;
        };

        match key {
            "pgfault" => page_faults = Some(value.parse()?),
            "pgmajfault" => major_page_faults = Some(value.parse()?),
            _ => {}
        }
    }

    Ok((page_faults.unwrap_or(0), major_page_faults.unwrap_or(0)))
}

fn parse_key_value_kb(contents: &str) -> HashMap<String, u64> {
    let mut values = HashMap::new();

    for line in contents.lines() {
        let mut fields = line.split_whitespace();

        let Some(raw_key) = fields.next() else {
            continue;
        };

        let Some(raw_value) = fields.next() else {
            continue;
        };

        let key = raw_key.trim_end_matches(':').to_string();

        if let Ok(value) = raw_value.parse::<u64>() {
            values.insert(key, value);
        }
    }

    values
}

fn get_required(values: &HashMap<String, u64>, key: &str) -> Result<u64> {
    values
        .get(key)
        .copied()
        .ok_or_else(|| anyhow!("missing {key} in /proc/meminfo"))
}

pub fn parse_macos_page_size(contents: &str) -> Option<u64> {
    let first_line = contents.lines().next()?;
    let (_, rest) = first_line.split_once("page size of ")?;
    let size = rest.split_whitespace().next()?;

    size.parse().ok()
}

pub fn parse_macos_vm_stat(contents: &str) -> HashMap<String, u64> {
    let mut values = HashMap::new();

    for line in contents.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let cleaned = value.trim().trim_end_matches('.').replace('.', "");

        if let Ok(value) = cleaned.parse::<u64>() {
            values.insert(key.trim().trim_matches('"').to_string(), value);
        }
    }

    values
}

pub fn parse_macos_swapusage(contents: &str) -> Result<(u64, u64)> {
    let fields: Vec<&str> = contents.split_whitespace().collect();

    let mut total_mb = 0;
    let mut used_mb = 0;

    for window in fields.windows(3) {
        if window[0] == "total" && window[1] == "=" {
            total_mb = parse_swap_mb(window[2]);
        } else if window[0] == "used" && window[1] == "=" {
            used_mb = parse_swap_mb(window[2]);
        }
    }

    Ok((total_mb.saturating_mul(1024), used_mb.saturating_mul(1024)))
}

fn parse_swap_mb(value: &str) -> u64 {
    value
        .trim_end_matches('M')
        .parse::<f64>()
        .map(|value| value as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo() {
        let input = "\
MemTotal:       16000000 kB
MemAvailable:   6000000 kB
SwapTotal:       2000000 kB
SwapFree:        1500000 kB
";

        let stats = parse_meminfo(input).unwrap();

        assert_eq!(stats.total_kb, 16_000_000);
        assert_eq!(stats.available_kb, 6_000_000);
        assert_eq!(stats.used_kb, 10_000_000);
        assert_eq!(stats.swap_total_kb, 2_000_000);
        assert_eq!(stats.swap_used_kb, 500_000);
        assert!((stats.utilization_percent - 62.5).abs() < 0.01);
    }

    #[test]
    fn parses_vmstat() {
        let input = "\
pgfault 12345
pgmajfault 67
";

        let (page_faults, major_page_faults) = parse_vmstat(input).unwrap();

        assert_eq!(page_faults, 12345);
        assert_eq!(major_page_faults, 67);
    }

    #[test]
    fn parses_macos_vm_stat() {
        let input = "\
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                4653.
Pages inactive:                          253010.
Pages speculative:                          208.
\"Translation faults\":                  18686622.
Pageins:                                1189566.
";

        let values = parse_macos_vm_stat(input);

        assert_eq!(parse_macos_page_size(input), Some(16384));
        assert_eq!(values.get("Pages free"), Some(&4653));
        assert_eq!(values.get("Translation faults"), Some(&18_686_622));
        assert_eq!(values.get("Pageins"), Some(&1_189_566));
    }

    #[test]
    fn parses_macos_swapusage() {
        let input = "total = 2048.00M  used = 512.00M  free = 1536.00M\n";

        let (total_kb, used_kb) = parse_macos_swapusage(input).unwrap();

        assert_eq!(total_kb, 2_097_152);
        assert_eq!(used_kb, 524_288);
    }
}

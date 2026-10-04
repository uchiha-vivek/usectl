use anyhow::{anyhow, Result};

use crate::pressure::{self, PressureStats};
use crate::utils;

#[derive(Debug, Clone)]
pub struct DiskStats {
    pub devices: Vec<DiskDevice>,
    pub pressure: Option<PressureStats>,
}

#[derive(Debug, Clone)]
pub struct DiskDevice {
    pub name: String,
    pub reads_completed: Option<u64>,
    pub sectors_read: Option<u64>,
    pub writes_completed: Option<u64>,
    pub sectors_written: Option<u64>,
    pub io_time_ms: Option<u64>,
    pub transfers: Option<u64>,
    pub mb_transferred: Option<f64>,
}

pub fn print_disk() -> Result<()> {
    let stats = read_disk_stats()?;

    println!("Disk");

    if stats.devices.is_empty() {
        println!("No physical block devices found");
    }

    for device in &stats.devices {
        println!("Disk: {}", device.name);
        println!();
        println!(
            "Reads Completed: {}",
            format_optional_u64(device.reads_completed)
        );
        println!(
            "Writes Completed: {}",
            format_optional_u64(device.writes_completed)
        );
        println!("Sectors Read: {}", format_optional_u64(device.sectors_read));
        println!(
            "Sectors Written: {}",
            format_optional_u64(device.sectors_written)
        );
        println!("I/O Time: {}", format_optional_ms(device.io_time_ms));

        if let Some(transfers) = device.transfers {
            println!("Transfers: {transfers}");
        }

        if let Some(mb_transferred) = device.mb_transferred {
            println!("MB Transferred: {mb_transferred:.2}");
        }

        println!();
    }

    pressure::print_pressure("IO PSI", &stats.pressure);

    Ok(())
}

pub fn read_disk_stats() -> Result<DiskStats> {
    utils::ensure_supported()?;

    if cfg!(target_os = "macos") {
        return read_macos_disk_stats();
    }

    Ok(DiskStats {
        devices: parse_diskstats(&utils::read_file("/proc/diskstats")?)?,
        pressure: pressure::read_pressure("/proc/pressure/io")?,
    })
}

fn read_macos_disk_stats() -> Result<DiskStats> {
    let iostat = utils::command_output("iostat", &["-Id", "1", "1"])?;

    Ok(DiskStats {
        devices: parse_macos_iostat(&iostat),
        pressure: None,
    })
}

pub fn parse_diskstats(contents: &str) -> Result<Vec<DiskDevice>> {
    let mut devices = Vec::new();

    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 14 {
            continue;
        }

        let name = fields[2];

        if is_pseudo_device(name) || is_partition(name) {
            continue;
        }

        devices.push(DiskDevice {
            name: name.to_string(),
            reads_completed: Some(parse_field(&fields, 3)?),
            sectors_read: Some(parse_field(&fields, 5)?),
            writes_completed: Some(parse_field(&fields, 7)?),
            sectors_written: Some(parse_field(&fields, 9)?),
            io_time_ms: Some(parse_field(&fields, 12)?),
            transfers: None,
            mb_transferred: None,
        });
    }

    Ok(devices)
}

fn parse_field(fields: &[&str], index: usize) -> Result<u64> {
    fields
        .get(index)
        .ok_or_else(|| anyhow!("missing /proc/diskstats field {index}"))?
        .parse()
        .map_err(Into::into)
}

fn is_pseudo_device(name: &str) -> bool {
    name.starts_with("loop")
        || name.starts_with("ram")
        || name.starts_with("fd")
        || name.starts_with("sr")
        || name.starts_with("dm-")
}

fn is_partition(name: &str) -> bool {
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        return name.contains('p') && name.chars().last().is_some_and(|c| c.is_ascii_digit());
    }

    name.chars().last().is_some_and(|c| c.is_ascii_digit())
}

fn format_optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "Not available".to_string(), |value| value.to_string())
}

fn format_optional_ms(value: Option<u64>) -> String {
    value.map_or_else(
        || "Not available".to_string(),
        |value| format!("{value} ms"),
    )
}

pub fn parse_macos_iostat(contents: &str) -> Vec<DiskDevice> {
    let mut lines = contents.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.find(|line| line.contains("disk")) else {
        return Vec::new();
    };

    let names: Vec<&str> = header.split_whitespace().collect();
    let Some(values_line) = lines.last() else {
        return Vec::new();
    };

    let values: Vec<&str> = values_line.split_whitespace().collect();
    let mut devices = Vec::new();

    for (index, name) in names.iter().enumerate() {
        let base = index * 3;

        if base + 2 >= values.len() {
            continue;
        }

        devices.push(DiskDevice {
            name: (*name).to_string(),
            reads_completed: None,
            writes_completed: None,
            sectors_read: None,
            sectors_written: None,
            io_time_ms: None,
            transfers: Some(values[base + 1].parse::<f64>().unwrap_or(0.0) as u64),
            mb_transferred: Some(values[base + 2].parse::<f64>().unwrap_or(0.0)),
        });
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diskstats_and_skips_pseudo_devices() {
        let input = "\
   7       0 loop0 1 0 2 3 4 0 5 6 0 7 8 0 0 0 0 0
   8       0 sda 10 0 20 0 30 0 40 0 0 50 60 0 0 0 0 0
   8       1 sda1 11 0 21 0 31 0 41 0 0 51 61 0 0 0 0 0
 259       0 nvme0n1 100 0 200 0 300 0 400 0 0 500 600 0 0 0 0 0
";

        let devices = parse_diskstats(input).unwrap();

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "sda");
        assert_eq!(devices[0].reads_completed, Some(10));
        assert_eq!(devices[0].sectors_read, Some(20));
        assert_eq!(devices[0].writes_completed, Some(30));
        assert_eq!(devices[0].sectors_written, Some(40));
        assert_eq!(devices[0].io_time_ms, Some(50));
        assert_eq!(devices[1].name, "nvme0n1");
    }

    #[test]
    fn parses_macos_iostat() {
        let input = "\
          disk0           disk1
    KB/t  xfrs   MB   KB/t  xfrs   MB
   32.00    10    1  64.00    20    2
";

        let devices = parse_macos_iostat(input);

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "disk0");
        assert_eq!(devices[0].reads_completed, None);
        assert_eq!(devices[0].transfers, Some(10));
        assert_eq!(devices[0].mb_transferred, Some(1.0));
        assert_eq!(devices[1].name, "disk1");
    }
}

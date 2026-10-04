use anyhow::{anyhow, Result};
use std::thread;
use std::time::Duration;

use crate::utils;

#[derive(Debug, Clone)]
pub struct CpuStats {
    pub utilization_percent: f64,
    pub load_1: f64,
    pub load_5: f64,
    pub load_15: f64,
    pub running_tasks: u64,
    pub total_tasks: u64,
}

#[derive(Debug, Clone)]
pub struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuTimes {
    fn idle_time(&self) -> u64 {
        self.idle + self.iowait
    }

    fn total_time(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }
}

pub fn print_cpu() -> Result<()> {
    let stats = read_cpu_stats()?;

    println!("CPU");
    println!(
        "Utilization: {}",
        utils::format_percent(stats.utilization_percent)
    );
    println!(
        "Load Average: {:.2} {:.2} {:.2}",
        stats.load_1, stats.load_5, stats.load_15
    );
    println!("Running Tasks: {}", stats.running_tasks);
    println!("Total Tasks: {}", stats.total_tasks);

    Ok(())
}

pub fn read_cpu_stats() -> Result<CpuStats> {
    utils::ensure_supported()?;

    if cfg!(target_os = "macos") {
        return read_macos_cpu_stats();
    }

    let first = parse_proc_stat(&utils::read_file("/proc/stat")?)?;

    // /proc/stat exposes cumulative CPU time counters.
    // Utilization must be calculated from the difference between two samples.
    thread::sleep(Duration::from_millis(250));

    let second = parse_proc_stat(&utils::read_file("/proc/stat")?)?;
    let utilization_percent = calculate_cpu_utilization(&first, &second);

    let (load_1, load_5, load_15, running_tasks, total_tasks) =
        parse_loadavg(&utils::read_file("/proc/loadavg")?)?;

    Ok(CpuStats {
        utilization_percent,
        load_1,
        load_5,
        load_15,
        running_tasks,
        total_tasks,
    })
}

fn read_macos_cpu_stats() -> Result<CpuStats> {
    let top = utils::command_output("top", &["-l", "2", "-n", "0", "-s", "1"])?;
    let utilization_percent = parse_macos_top_cpu(&top).unwrap_or(0.0);
    let (running_tasks, total_tasks) = parse_macos_top_processes(&top).unwrap_or((0, 0));
    let (load_1, load_5, load_15) = parse_macos_uptime(&utils::command_output("uptime", &[])?)?;

    Ok(CpuStats {
        utilization_percent,
        load_1,
        load_5,
        load_15,
        running_tasks,
        total_tasks,
    })
}

pub fn parse_proc_stat(contents: &str) -> Result<CpuTimes> {
    let cpu_line = contents
        .lines()
        .find(|line| line.starts_with("cpu "))
        .ok_or_else(|| anyhow!("missing aggregate cpu line in /proc/stat"))?;

    let fields: Vec<&str> = cpu_line.split_whitespace().collect();

    if fields.len() < 8 {
        return Err(anyhow!("invalid cpu line in /proc/stat"));
    }

    Ok(CpuTimes {
        user: fields[1].parse()?,
        nice: fields[2].parse()?,
        system: fields[3].parse()?,
        idle: fields[4].parse()?,
        iowait: fields[5].parse()?,
        irq: fields[6].parse()?,
        softirq: fields[7].parse()?,
        steal: fields.get(8).unwrap_or(&"0").parse()?,
    })
}

pub fn parse_loadavg(contents: &str) -> Result<(f64, f64, f64, u64, u64)> {
    let fields: Vec<&str> = contents.split_whitespace().collect();

    if fields.len() < 4 {
        return Err(anyhow!("invalid /proc/loadavg contents"));
    }

    let load_1 = fields[0].parse()?;
    let load_5 = fields[1].parse()?;
    let load_15 = fields[2].parse()?;

    let (running, total) = fields[3]
        .split_once('/')
        .ok_or_else(|| anyhow!("invalid task counts in /proc/loadavg"))?;

    Ok((load_1, load_5, load_15, running.parse()?, total.parse()?))
}

pub fn calculate_cpu_utilization(first: &CpuTimes, second: &CpuTimes) -> f64 {
    let total_delta = second.total_time().saturating_sub(first.total_time());
    let idle_delta = second.idle_time().saturating_sub(first.idle_time());

    if total_delta == 0 {
        return 0.0;
    }

    let busy_delta = total_delta.saturating_sub(idle_delta);

    busy_delta as f64 / total_delta as f64 * 100.0
}

pub fn parse_macos_top_cpu(contents: &str) -> Option<f64> {
    contents
        .lines()
        .rev()
        .find(|line| line.starts_with("CPU usage:"))
        .and_then(|line| {
            let idle = line
                .split(',')
                .find_map(|part| part.trim().strip_suffix(" idle"))
                .and_then(|part| part.split_whitespace().next())
                .and_then(|value| value.trim_end_matches('%').parse::<f64>().ok())?;

            Some(100.0 - idle)
        })
}

pub fn parse_macos_top_processes(contents: &str) -> Option<(u64, u64)> {
    contents
        .lines()
        .rev()
        .find(|line| line.starts_with("Processes:"))
        .and_then(|line| {
            let total = line
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse::<u64>().ok())?;

            let running = line.split(',').find_map(|part| {
                let part = part.trim();
                if part.ends_with("running") {
                    part.split_whitespace().next()?.parse::<u64>().ok()
                } else {
                    None
                }
            })?;

            Some((running, total))
        })
}

pub fn parse_macos_uptime(contents: &str) -> Result<(f64, f64, f64)> {
    let marker = "load averages:";
    let (_, loads) = contents
        .split_once(marker)
        .ok_or_else(|| anyhow!("missing load averages in uptime output"))?;

    let fields: Vec<&str> = loads.split_whitespace().collect();

    if fields.len() < 3 {
        return Err(anyhow!("invalid uptime load averages"));
    }

    Ok((fields[0].parse()?, fields[1].parse()?, fields[2].parse()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_stat() {
        let input = "cpu  100 0 40 1000 10 0 5 2 0 0\n";

        let stats = parse_proc_stat(input).unwrap();

        assert_eq!(stats.user, 100);
        assert_eq!(stats.nice, 0);
        assert_eq!(stats.system, 40);
        assert_eq!(stats.idle, 1000);
        assert_eq!(stats.iowait, 10);
        assert_eq!(stats.irq, 0);
        assert_eq!(stats.softirq, 5);
        assert_eq!(stats.steal, 2);
    }

    #[test]
    fn parses_loadavg() {
        let input = "0.82 0.71 0.65 2/312 12345\n";

        let (load_1, load_5, load_15, running, total) = parse_loadavg(input).unwrap();

        assert_eq!(load_1, 0.82);
        assert_eq!(load_5, 0.71);
        assert_eq!(load_15, 0.65);
        assert_eq!(running, 2);
        assert_eq!(total, 312);
    }

    #[test]
    fn calculates_cpu_utilization_from_deltas() {
        let first = CpuTimes {
            user: 100,
            nice: 0,
            system: 50,
            idle: 800,
            iowait: 50,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let second = CpuTimes {
            user: 150,
            nice: 0,
            system: 70,
            idle: 870,
            iowait: 60,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let utilization = calculate_cpu_utilization(&first, &second);

        assert!((utilization - 46.666).abs() < 0.01);
    }

    #[test]
    fn parses_macos_top_cpu() {
        let input = "\
Processes: 400 total, 2 running, 398 sleeping
CPU usage: 7.10% user, 8.20% sys, 84.70% idle
";

        let utilization = parse_macos_top_cpu(input).unwrap();

        assert!((utilization - 15.30).abs() < 0.01);
    }

    #[test]
    fn parses_macos_uptime() {
        let input = "13:01  1 user, load averages: 2.61 1.91 1.81\n";

        let (load_1, load_5, load_15) = parse_macos_uptime(input).unwrap();

        assert_eq!(load_1, 2.61);
        assert_eq!(load_5, 1.91);
        assert_eq!(load_15, 1.81);
    }
}

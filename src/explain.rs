use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Explanation {
    pub topic: &'static str,
    pub summary: &'static str,
    pub details: &'static [&'static str],
    pub useful_when: &'static [&'static str],
}

pub fn print_explanation(topic: &str) -> Result<()> {
    let explanation = explain_topic(topic)?;

    println!("{}", explanation.topic);
    println!("{}", "=".repeat(explanation.topic.len()));
    println!();
    println!("{}", explanation.summary);
    println!();
    println!("Details:");
    for detail in explanation.details {
        println!("- {detail}");
    }
    println!();
    println!("Useful when:");
    for item in explanation.useful_when {
        println!("- {item}");
    }

    Ok(())
}

pub fn explain_topic(topic: &str) -> Result<Explanation> {
    let normalized = normalize_topic(topic);

    match normalized.as_str() {
        "load-average" | "load" => Ok(Explanation {
            topic: "Load Average",
            summary: "Load average is the average number of tasks that are running or waiting to run.",
            details: &[
                "The three values usually represent 1-minute, 5-minute, and 15-minute averages.",
                "It is not the same as CPU utilization.",
                "A high load with low CPU utilization can point to tasks waiting on disk, network, or locks.",
                "Interpret load relative to CPU/core count and system workload.",
            ],
            useful_when: &[
                "Checking whether work is piling up faster than the system can handle it.",
                "Comparing short-term load against longer-term load to see whether pressure is rising or falling.",
            ],
        }),
        "psi" | "pressure" | "pressure-stall-information" => Ok(Explanation {
            topic: "PSI",
            summary: "PSI means Pressure Stall Information, a Linux signal for time lost because tasks are waiting on CPU, memory, or IO.",
            details: &[
                "Linux exposes PSI through /proc/pressure/cpu, /proc/pressure/memory, and /proc/pressure/io.",
                "The some line means at least one task was stalled.",
                "The full line means all non-idle tasks were stalled at the same time.",
                "avg10, avg60, and avg300 are recent pressure percentages over 10, 60, and 300 seconds.",
                "macOS does not expose Linux PSI, so usectl reports it as unavailable there.",
            ],
            useful_when: &[
                "Understanding saturation instead of only utilization.",
                "Seeing whether memory or IO pressure is causing applications to wait.",
            ],
        }),
        "cpu-utilization" | "cpu" => Ok(Explanation {
            topic: "CPU Utilization",
            summary: "CPU utilization is the percentage of sampled CPU time spent doing non-idle work.",
            details: &[
                "On Linux, /proc/stat contains cumulative CPU counters.",
                "usectl samples those counters twice and calculates utilization from the delta.",
                "A single raw /proc/stat sample is not enough because the counters are totals since boot.",
                "On macOS, usectl reads native top output for a comparable snapshot.",
            ],
            useful_when: &[
                "Checking whether the CPU is actually busy.",
                "Separating high load from high CPU usage.",
            ],
        }),
        "memory-utilization" | "memory" | "memavailable" => Ok(Explanation {
            topic: "Memory Utilization",
            summary: "Memory utilization estimates how much RAM is used while treating reclaimable memory carefully.",
            details: &[
                "On Linux, usectl uses MemTotal - MemAvailable.",
                "MemAvailable is better than subtracting free memory because Linux uses cache aggressively.",
                "Cached memory is often useful and reclaimable, so it should not automatically be counted as unavailable.",
                "On macOS, usectl estimates availability from vm_stat page counters.",
            ],
            useful_when: &[
                "Checking whether memory is genuinely constrained.",
                "Avoiding false alarms caused by normal filesystem cache usage.",
            ],
        }),
        "swap" => Ok(Explanation {
            topic: "Swap",
            summary: "Swap is disk-backed memory used when the operating system moves memory pages out of RAM.",
            details: &[
                "Some swap usage can be normal, especially on long-running desktop systems.",
                "Growing swap usage plus poor responsiveness can indicate memory pressure.",
                "Swap is much slower than RAM, even on fast SSDs.",
            ],
            useful_when: &[
                "Investigating slow applications or system responsiveness.",
                "Understanding whether memory pressure has pushed pages out of RAM.",
            ],
        }),
        "page-faults" | "faults" | "major-faults" => Ok(Explanation {
            topic: "Page Faults",
            summary: "A page fault happens when a program accesses memory that is not currently mapped for immediate use.",
            details: &[
                "Minor page faults are often normal and do not require disk IO.",
                "Major page faults usually require reading data from storage.",
                "The counters shown by usectl are cumulative, so trends are more useful than one isolated number.",
            ],
            useful_when: &[
                "Checking whether applications are frequently waiting on memory-backed storage activity.",
                "Comparing fault counters over time with watch mode.",
            ],
        }),
        "disk-transfers" | "transfers" | "iostat" => Ok(Explanation {
            topic: "Disk Transfers",
            summary: "Disk transfers are completed disk operations reported by the platform's disk statistics source.",
            details: &[
                "On Linux, usectl reports /proc/diskstats counters such as reads, writes, sectors, and IO time.",
                "On macOS, usectl uses iostat and reports transfers completed plus MB transferred.",
                "macOS iostat does not provide the same simple read/write split as Linux /proc/diskstats in this version.",
            ],
            useful_when: &[
                "Checking whether storage activity exists.",
                "Understanding why some disk fields are unavailable on macOS.",
            ],
        }),
        "network-drops" | "drops" | "network-errors" | "errors" => Ok(Explanation {
            topic: "Network Drops and Errors",
            summary: "Network drops and errors are interface counters that can indicate packet loss or device-level problems.",
            details: &[
                "Drops mean packets were discarded before successful delivery through the interface.",
                "Errors can indicate malformed packets, hardware issues, driver issues, or link problems.",
                "Small counters may be historical; rising counters during watch mode are more meaningful.",
            ],
            useful_when: &[
                "Investigating packet loss or unstable connectivity.",
                "Checking whether network saturation or interface problems are visible locally.",
            ],
        }),
        "use" | "use-methodology" => Ok(Explanation {
            topic: "USE Methodology",
            summary: "USE means Utilization, Saturation, and Errors, a practical way to inspect system resources.",
            details: &[
                "Utilization asks how busy a resource is.",
                "Saturation asks whether work is queuing or waiting.",
                "Errors asks whether the resource is reporting failures.",
                "usectl maps these ideas onto CPU, memory, disk, and network signals.",
            ],
            useful_when: &[
                "Building a systematic checklist instead of guessing.",
                "Learning what low-level operating system counters can and cannot tell you.",
            ],
        }),
        _ => bail!(
            "unknown explanation topic '{topic}'. Try one of: load-average, psi, cpu, memory, swap, page-faults, disk-transfers, network-drops, use"
        ),
    }
}

fn normalize_topic(topic: &str) -> String {
    topic.trim().to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_known_topic_alias() {
        let explanation = explain_topic("load").unwrap();

        assert_eq!(explanation.topic, "Load Average");
    }

    #[test]
    fn rejects_unknown_topic() {
        assert!(explain_topic("definitely-not-a-topic").is_err());
    }
}

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::utils;

#[derive(Debug, Clone, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_drops: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_drops: u64,
}

pub fn print_network() -> Result<()> {
    let interfaces = read_network_stats()?;

    println!("Network");

    if interfaces.is_empty() {
        println!("No non-loopback network interfaces found");
    }

    for interface in interfaces {
        println!("Interface: {}", interface.name);
        println!();
        println!("RX Bytes: {}", interface.rx_bytes);
        println!("TX Bytes: {}", interface.tx_bytes);
        println!();
        println!("RX Packets: {}", interface.rx_packets);
        println!("TX Packets: {}", interface.tx_packets);
        println!();
        println!("RX Errors: {}", interface.rx_errors);
        println!("TX Errors: {}", interface.tx_errors);
        println!();
        println!("RX Drops: {}", interface.rx_drops);
        println!("TX Drops: {}", interface.tx_drops);
        println!();
    }

    Ok(())
}

pub fn read_network_stats() -> Result<Vec<NetworkInterface>> {
    utils::ensure_supported()?;

    if cfg!(target_os = "macos") {
        return parse_macos_netstat(&utils::command_output("netstat", &["-ibn"])?);
    }

    parse_net_dev(&utils::read_file("/proc/net/dev")?)
}

pub fn parse_net_dev(contents: &str) -> Result<Vec<NetworkInterface>> {
    let mut interfaces = Vec::new();

    for line in contents.lines().skip(2) {
        let Some((raw_name, raw_values)) = line.split_once(':') else {
            continue;
        };

        let name = raw_name.trim();

        if name == "lo" {
            continue;
        }

        let values: Vec<&str> = raw_values.split_whitespace().collect();

        if values.len() < 16 {
            return Err(anyhow!("invalid /proc/net/dev line for {name}"));
        }

        interfaces.push(NetworkInterface {
            name: name.to_string(),
            rx_bytes: values[0].parse()?,
            rx_packets: values[1].parse()?,
            rx_errors: values[2].parse()?,
            rx_drops: values[3].parse()?,
            tx_bytes: values[8].parse()?,
            tx_packets: values[9].parse()?,
            tx_errors: values[10].parse()?,
            tx_drops: values[11].parse()?,
        });
    }

    Ok(interfaces)
}

pub fn parse_macos_netstat(contents: &str) -> Result<Vec<NetworkInterface>> {
    let mut interfaces = Vec::new();

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 10 {
            continue;
        }

        let name = fields[0];

        if name == "lo0"
            || name.ends_with('*')
            || is_macos_pseudo_interface(name)
            || !fields
                .get(2)
                .is_some_and(|field| field.starts_with("<Link"))
            || interfaces
                .iter()
                .any(|iface: &NetworkInterface| iface.name == name)
        {
            continue;
        }

        let base = if fields
            .get(3)
            .is_some_and(|value| value.parse::<u64>().is_ok())
        {
            3
        } else {
            4
        };

        if fields.len() <= base + 5 {
            continue;
        }

        let rx_packets = fields[base].parse()?;
        let rx_errors = fields[base + 1].parse()?;
        let rx_bytes = fields[base + 2].parse()?;
        let tx_packets = fields[base + 3].parse()?;
        let tx_errors = fields[base + 4].parse()?;
        let tx_bytes = fields[base + 5].parse()?;

        if rx_packets == 0 && rx_bytes == 0 && tx_packets == 0 && tx_bytes == 0 {
            continue;
        }

        interfaces.push(NetworkInterface {
            name: name.to_string(),
            rx_packets,
            rx_errors,
            rx_bytes,
            tx_packets,
            tx_errors,
            tx_bytes,
            rx_drops: 0,
            tx_drops: 0,
        });
    }

    Ok(interfaces)
}

fn is_macos_pseudo_interface(name: &str) -> bool {
    name.starts_with("awdl")
        || name.starts_with("llw")
        || name.starts_with("utun")
        || name.starts_with("bridge")
        || name.starts_with("anpi")
        || name.starts_with("ap")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_net_dev_and_skips_loopback() {
        let input = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 100 1 0 0 0 0 0 0 200 2 0 0 0 0 0 0
  eth0: 1234 10 1 2 0 0 0 0 5678 20 3 4 0 0 0 0
";

        let interfaces = parse_net_dev(input).unwrap();

        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "eth0");
        assert_eq!(interfaces[0].rx_bytes, 1234);
        assert_eq!(interfaces[0].rx_packets, 10);
        assert_eq!(interfaces[0].rx_errors, 1);
        assert_eq!(interfaces[0].rx_drops, 2);
        assert_eq!(interfaces[0].tx_bytes, 5678);
        assert_eq!(interfaces[0].tx_packets, 20);
        assert_eq!(interfaces[0].tx_errors, 3);
        assert_eq!(interfaces[0].tx_drops, 4);
    }

    #[test]
    fn parses_macos_netstat() {
        let input = "\
Name       Mtu   Network       Address            Ipkts Ierrs     Ibytes    Opkts Oerrs     Obytes  Coll
lo0        16384 <Link#1>                         10    0         100       20    0         200     0
en0        1500  <Link#12>                        30    1         3000      40    2         4000    0
";

        let interfaces = parse_macos_netstat(input).unwrap();

        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "en0");
        assert_eq!(interfaces[0].rx_packets, 30);
        assert_eq!(interfaces[0].rx_errors, 1);
        assert_eq!(interfaces[0].rx_bytes, 3000);
        assert_eq!(interfaces[0].tx_packets, 40);
        assert_eq!(interfaces[0].tx_errors, 2);
        assert_eq!(interfaces[0].tx_bytes, 4000);
    }
}

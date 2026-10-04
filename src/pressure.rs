use anyhow::Result;
use serde::Serialize;

use crate::utils;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PressureStats {
    pub some_avg10: Option<f64>,
    pub some_avg60: Option<f64>,
    pub some_avg300: Option<f64>,
    pub full_avg10: Option<f64>,
    pub full_avg60: Option<f64>,
    pub full_avg300: Option<f64>,
}

pub fn read_pressure(path: &str) -> Result<Option<PressureStats>> {
    match utils::read_file(path) {
        Ok(contents) => Ok(Some(parse_pressure(&contents))),
        Err(_) => Ok(None),
    }
}

pub fn parse_pressure(contents: &str) -> PressureStats {
    let mut stats = PressureStats::default();

    for line in contents.lines() {
        let mut parts = line.split_whitespace();

        let Some(kind) = parts.next() else {
            continue;
        };

        for field in parts {
            let Some((key, value)) = field.split_once('=') else {
                continue;
            };

            let Ok(value) = value.parse::<f64>() else {
                continue;
            };

            match (kind, key) {
                ("some", "avg10") => stats.some_avg10 = Some(value),
                ("some", "avg60") => stats.some_avg60 = Some(value),
                ("some", "avg300") => stats.some_avg300 = Some(value),
                ("full", "avg10") => stats.full_avg10 = Some(value),
                ("full", "avg60") => stats.full_avg60 = Some(value),
                ("full", "avg300") => stats.full_avg300 = Some(value),
                _ => {}
            }
        }
    }

    stats
}

pub fn print_pressure(label: &str, pressure: &Option<PressureStats>) {
    println!("{label}:");

    match pressure {
        Some(stats) => {
            if let Some(avg10) = stats.some_avg10 {
                println!("some avg10={avg10:.2}");
            } else {
                println!("some avg10=Not available");
            }

            if let Some(avg10) = stats.full_avg10 {
                println!("full avg10={avg10:.2}");
            } else {
                println!("full avg10=Not available");
            }
        }
        None => println!("unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pressure_stats() {
        let input = "\
some avg10=1.20 avg60=0.50 avg300=0.10 total=123
full avg10=0.30 avg60=0.20 avg300=0.01 total=45
";

        let stats = parse_pressure(input);

        assert_eq!(stats.some_avg10, Some(1.20));
        assert_eq!(stats.some_avg60, Some(0.50));
        assert_eq!(stats.some_avg300, Some(0.10));
        assert_eq!(stats.full_avg10, Some(0.30));
        assert_eq!(stats.full_avg60, Some(0.20));
        assert_eq!(stats.full_avg300, Some(0.01));
    }
}

//! Analyze command implementation

use anyhow::Result;
use crate::replay::{ProfileReader, DistributionAnalyzer};

pub fn run(input: &str) -> Result<()> {
    let reader = ProfileReader::new(input)?;
    let metadata = reader.metadata();
    let analysis = DistributionAnalyzer::analyze(reader.events());

    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║            Profile Analysis Report                    ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // File metadata
    println!("Profile: {}", input);
    println!("Version: {}\n", metadata.version);

    // Event statistics
    println!("─ Event Statistics ─");
    println!("Total events: {}", analysis.total_events);
    println!("Unique connections: {}\n", metadata.unique_connections);

    // Time range
    let time_range = metadata.time_range;
    if time_range.0 > 0 || time_range.1 > 0 {
        let duration_micros = time_range.1.saturating_sub(time_range.0);
        let duration_secs = duration_micros as f64 / 1_000_000.0;
        println!("Time range: {:.2} seconds\n", duration_secs);
    }

    // Command distribution
    println!("─ Command Distribution ─");
    let mut cmd_entries: Vec<_> = analysis.command_distribution.iter().collect();
    cmd_entries.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (cmd, count) in cmd_entries {
        let percentage = (*count as f64 / analysis.total_events as f64) * 100.0;
        println!("{:?}: {} ({:.1}%)", cmd, count, percentage);
    }

    // Key size distribution
    println!("\n─ Key Size Distribution ─");
    if !analysis.key_size_distribution.is_empty() {
        let mut key_sizes: Vec<_> = analysis.key_size_distribution.clone();
        key_sizes.sort_by_key(|(size, _)| *size);

        let total_keys = analysis.total_events;
        let min_size = key_sizes.iter().map(|(s, _)| *s).min().unwrap_or(0);
        let max_size = key_sizes.iter().map(|(s, _)| *s).max().unwrap_or(0);

        let avg_size: f64 = key_sizes.iter()
            .map(|(size, count)| *size as f64 * *count as f64)
            .sum::<f64>() / total_keys.max(1) as f64;

        println!("Min: {} bytes", min_size);
        println!("Max: {} bytes", max_size);
        println!("Avg: {:.1} bytes", avg_size);

        if key_sizes.len() <= 10 {
            println!("\nDistribution:");
            for (size, count) in &key_sizes {
                let percentage = (*count as f64 / total_keys as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        } else {
            println!("\nTop 10 sizes:");
            let mut top_sizes = key_sizes.clone();
            top_sizes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (size, count) in top_sizes.iter().take(10) {
                let percentage = (*count as f64 / total_keys as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        }
    }

    // Value size distribution
    println!("\n─ Value Size Distribution ─");
    if !analysis.value_size_distribution.is_empty() {
        let mut value_sizes: Vec<_> = analysis.value_size_distribution.clone();
        value_sizes.sort_by_key(|(size, _)| *size);

        let total_values = value_sizes.iter().map(|(_, c)| c).sum::<u64>();
        let min_size = value_sizes.iter().map(|(s, _)| *s).min().unwrap_or(0);
        let max_size = value_sizes.iter().map(|(s, _)| *s).max().unwrap_or(0);

        let avg_size: f64 = value_sizes.iter()
            .map(|(size, count)| *size as f64 * *count as f64)
            .sum::<f64>() / total_values.max(1) as f64;

        println!("Min: {} bytes", min_size);
        println!("Max: {} bytes", max_size);
        println!("Avg: {:.1} bytes", avg_size);
        println!("Total with values: {} ({:.1}%)", total_values, (total_values as f64 / analysis.total_events as f64) * 100.0);

        if value_sizes.len() <= 10 {
            println!("\nDistribution:");
            for (size, count) in &value_sizes {
                let percentage = (*count as f64 / total_values as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        } else {
            println!("\nTop 10 sizes:");
            let mut top_sizes = value_sizes.clone();
            top_sizes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (size, count) in top_sizes.iter().take(10) {
                let percentage = (*count as f64 / total_values as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        }
    } else {
        println!("No value data in profile");
    }

    println!("\n");

    Ok(())
}

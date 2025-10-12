pub mod cli;
pub mod measure;

use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Args)]
pub struct MeasureArgs {
    /// Number of measurement iterations
    #[arg(short, long, default_value_t = 1)]
    iterations: u8,
    /// Seconds to sleep between each successful measurement
    #[arg(short, long, default_value_t = 0)]
    sleep: u8,
    /// Paths to scenario files to measure
    #[arg(required = true, num_args = 1..)]
    scenarios: Vec<PathBuf>,
    /// Enable RAPL package energy measurement (entire CPU socket)
    #[arg(long)]
    rapl_pkg: bool,
    /// Enable RAPL core energy measurement (CPU cores only)
    #[arg(long)]
    rapl_cores: bool,
    /// Enable RAPL GPU energy measurement (integrated graphics)
    #[arg(long)]
    rapl_gpu: bool,
    /// Enable RAPL DRAM energy measurement (system memory)
    #[arg(long)]
    rapl_dram: bool,
    /// Enable RAPL platform energy measurement (entire SoC)
    #[arg(long)]
    rapl_psys: bool,
    /// Enable all supported and available RAPL energy measurements
    #[arg(long)]
    rapl_all: bool,
    /// Enable hardware CPU cycle counting
    #[arg(long)]
    hw_cycles: bool,
    /// Enable hardware cache miss counting
    #[arg(long)]
    hw_cache_misses: bool,
    /// Enable hardware branch misprediction counting
    #[arg(long)]
    hw_branch_misses: bool,
    /// Enable all supported and available hardware performance counters
    #[arg(long)]
    hw_all: bool,
    /// Output file path for CSV measurement results
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct Measurement {
    language: String,
    scenario: String,
    test: String,
    warmup: bool,
    pkg: Option<f64>,
    cores: Option<f64>,
    gpu: Option<f64>,
    dram: Option<f64>,
    psys: Option<f64>,
    cycles: Option<u64>,
    cache_misses: Option<u64>,
    branch_misses: Option<u64>,
    iteration: usize,
    timestamp: i64,
}

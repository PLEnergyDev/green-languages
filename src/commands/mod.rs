pub mod measure;

use crate::core::MeasurementMode;
use clap::{Args, Parser};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub measure: MeasureArgs,
}

#[derive(Args)]
pub struct MeasureArgs {
    /// Number of measurement iterations
    #[arg(short, long, default_value_t = 1)]
    pub iterations: u8,
    /// Seconds to sleep between each successful measurement
    #[arg(short, long, default_value_t = 0)]
    pub sleep: u8,
    /// Paths to scenario files to measure
    #[arg(required = true, num_args = 1..)]
    pub scenarios: Vec<PathBuf>,
    /// Enable RAPL package energy measurement (entire CPU socket)
    #[arg(long)]
    pub rapl_pkg: bool,
    /// Enable RAPL core energy measurement (CPU cores only)
    #[arg(long)]
    pub rapl_cores: bool,
    /// Enable RAPL GPU energy measurement (integrated graphics)
    #[arg(long)]
    pub rapl_gpu: bool,
    /// Enable RAPL DRAM energy measurement (system memory)
    #[arg(long)]
    pub rapl_dram: bool,
    /// Enable RAPL platform energy measurement (entire SoC)
    #[arg(long)]
    pub rapl_psys: bool,
    /// Enable all supported and available RAPL energy measurements
    #[arg(long)]
    pub rapl_all: bool,
    /// Enable hardware CPU cycle counting
    #[arg(long)]
    pub hw_cycles: bool,
    /// Enable hardware cache miss counting
    #[arg(long)]
    pub hw_cache_misses: bool,
    /// Enable hardware branch misprediction counting
    #[arg(long)]
    pub hw_branch_misses: bool,
    /// Enable all supported and available hardware performance counters
    #[arg(short, long)]
    pub time: bool,
    #[arg(long)]
    pub hw_all: bool,
    /// Output file path for CSV measurement results
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct Measurement {
    pub scenario: String,
    pub language: String,
    pub test: String,
    pub mode: MeasurementMode,
    /// Index of performed measurement
    pub iteration: usize,
    /// Elapsed time in nanoseconds
    pub time: Option<u64>,
    pub pkg: Option<f64>,
    pub cores: Option<f64>,
    pub gpu: Option<f64>,
    pub dram: Option<f64>,
    pub psys: Option<f64>,
    pub cycles: Option<u64>,
    pub l1d_misses: Option<u64>,
    pub l1i_misses: Option<u64>,
    pub llc_misses: Option<u64>,
    pub branch_misses: Option<u64>,
    pub ended: i64,
}

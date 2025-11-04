pub mod config;
pub mod core;
pub mod measure;

use crate::core::MeasurementMode;
use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
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
    /// ALL available RAPL energy domains
    #[arg(long)]
    pub rapl: bool,
    /// CPU cycle count
    #[arg(long)]
    pub cycles: bool,
    /// Cache miss count
    #[arg(long)]
    pub cache_misses: bool,
    /// Branch misprediction count
    #[arg(long)]
    pub branch_misses: bool,
    /// All available low power C-states
    #[arg(long)]
    pub cstates: bool,
    /// Elapsed time
    #[arg(short, long)]
    pub time: bool,
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
    pub iteration: usize,
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

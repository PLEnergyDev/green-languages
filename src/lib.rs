pub mod core;
pub mod measure;

use crate::core::MeasurementMode;
use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct MeasureCommand {
    /// Number of measurement iterations
    #[arg(short, long, default_value_t = 1)]
    pub iterations: u8,
    /// Sleep seconds between each successful measurement
    #[arg(short, long, default_value_t = 0)]
    pub sleep: u8,
    /// YAML scenario paths to measure
    #[arg(required = true, num_args = 1..)]
    pub scenarios: Vec<PathBuf>,
    /// Measure all available RAPL energy domains
    #[arg(long)]
    pub rapl: bool,
    /// Measure elapsed time and CPU cycle count
    #[arg(long)]
    pub cycles: bool,
    /// Measure cache loads miss and branch misprediction count
    #[arg(long)]
    pub misses: bool,
    /// Measure and sum all available low power C-states
    #[arg(long)]
    pub cstates: bool,
    /// CSV measurements output path
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct Measurement {
    pub scenario: String,
    pub language: String,
    pub test: String,
    pub nice: Option<i32>,
    pub affinity: Option<String>,
    pub mode: MeasurementMode,
    pub iteration: usize,
    pub time: Option<u64>,
    pub pkg: Option<f64>,
    pub cores: Option<f64>,
    pub gpu: Option<f64>,
    pub ram: Option<f64>,
    pub psys: Option<f64>,
    pub cycles: Option<u64>,
    pub l1d_misses: Option<u64>,
    pub l1i_misses: Option<u64>,
    pub llc_misses: Option<u64>,
    pub branch_misses: Option<u64>,
    pub c1_core_residency: Option<u64>,
    pub c3_core_residency: Option<u64>,
    pub c6_core_residency: Option<u64>,
    pub c7_core_residency: Option<u64>,
    pub c2_pkg_residency: Option<u64>,
    pub c3_pkg_residency: Option<u64>,
    pub c6_pkg_residency: Option<u64>,
    pub c8_pkg_residency: Option<u64>,
    pub c10_pkg_residency: Option<u64>,
    pub ended: i64,
}

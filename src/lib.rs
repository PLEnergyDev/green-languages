pub mod core;
pub mod measure;

use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;

use crate::core::MeasurementMode;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct MeasureCommand {
    /// YAML scenario paths to measure
    #[arg(required = true, num_args = 1..)]
    pub scenarios: Vec<PathBuf>,
    /// Output directory for build artifacts and measurements
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RawMeasurement {
    pub time: Option<u64>,
    pub pkg: Option<f64>,
    pub cores: Option<f64>,
    pub gpu: Option<f64>,
    pub psys: Option<f64>,
    pub cycles: Option<u64>,
    pub l1d_misses: Option<u64>,
    pub l1i_misses: Option<u64>,
    pub llc_misses: Option<u64>,
    pub branch_misses: Option<u64>,
    pub c1_core_residency: Option<u64>,
    pub c6_core_residency: Option<u64>,
    pub c7_core_residency: Option<u64>,
    pub c2_pkg_residency: Option<u64>,
    pub c3_pkg_residency: Option<u64>,
    pub c6_pkg_residency: Option<u64>,
    pub c8_pkg_residency: Option<u64>,
    pub c10_pkg_residency: Option<u64>,
    pub ended: i64,
}

#[derive(Debug, Serialize)]
pub struct Measurement {
    pub scenario: String,
    pub language: String,
    pub test: String,
    pub nice: Option<i32>,
    pub affinity: Option<String>,
    pub mode: MeasurementMode,
    pub run: usize,
    pub time: Option<u64>,
    pub pkg: Option<f64>,
    pub cores: Option<f64>,
    pub gpu: Option<f64>,
    pub psys: Option<f64>,
    pub cycles: Option<u64>,
    pub l1d_misses: Option<u64>,
    pub l1i_misses: Option<u64>,
    pub llc_misses: Option<u64>,
    pub branch_misses: Option<u64>,
    pub c1_core_residency: Option<u64>,
    pub c6_core_residency: Option<u64>,
    pub c7_core_residency: Option<u64>,
    pub c2_pkg_residency: Option<u64>,
    pub c3_pkg_residency: Option<u64>,
    pub c6_pkg_residency: Option<u64>,
    pub c8_pkg_residency: Option<u64>,
    pub c10_pkg_residency: Option<u64>,
    pub ended: i64,
}

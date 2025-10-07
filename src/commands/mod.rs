pub mod cli;
pub mod measure;
pub mod report;

use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Args)]
pub struct MeasureArgs {
    #[arg(short, long, default_value_t = 1)]
    iterations: u8,
    #[arg(short, long, default_value_t = 100)]
    frequency: u16,
    #[arg(short, long)]
    sleep: Option<u8>,
    #[arg(short, long, default_value_t = 100)]
    timeout: u8,
    #[arg(required = true, num_args = 1..)]
    scenarios: Vec<PathBuf>,
    #[arg(long)]
    rapl_pkg: bool,
    #[arg(long)]
    rapl_cores: bool,
    #[arg(long)]
    rapl_gpu: bool,
    #[arg(long)]
    rapl_dram: bool,
    #[arg(long)]
    rapl_psys: bool,
    #[arg(long)]
    rapl_all: bool,
    #[arg(long, default_value = "results.csv")]
    output: PathBuf,
    #[arg(long)]
    affinity: Option<usize>,
    #[arg(long)]
    niceness: Option<i32>,
    #[arg(long)]
    warmup: bool,
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
    iteration: usize,
    timestamp: i64,
}

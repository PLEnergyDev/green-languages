use clap::{Parser, Subcommand};
use super::measure::MeasureArgs;
// use super::report::ReportArgs;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Measure(MeasureArgs),
    // Report(ReportArgs),
}

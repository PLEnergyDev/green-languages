use clap::Parser;
use flexi_logger::{DeferredNow, FileSpec, Logger, Record, WriteMode};
use green_languages::commands::cli::{Cli, Commands};
use green_languages::commands::measure::run;
use green_languages::config::Config;
use std::io::{Error, Write};

fn custom_format(w: &mut dyn Write, now: &mut DeferredNow, record: &Record) -> Result<(), Error> {
    write!(
        w,
        "{} {} {}",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        record.level(),
        record.args()
    )
}

fn configure_logger() -> Result<(), Box<dyn std::error::Error>> {
    let results_dir = Config::global().base_dir.join("results");
    let file_spec = FileSpec::default()
        .directory(results_dir)
        .basename("gl")
        .suppress_timestamp();
    Logger::try_with_str("info")?
        .log_to_file(file_spec)
        .write_mode(WriteMode::Direct)
        .format(custom_format)
        .append()
        .start()?;
    Ok(())
}

fn main() {
    Config::global();
    let cli = Cli::parse();
    if let Err(err) = configure_logger() {
        eprintln!("Failed to configure logger: {}", err);
        std::process::exit(1);
    }
    let result = match cli.command {
        Commands::Measure(args) => run(args),
    };
    if let Err(err) = result {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

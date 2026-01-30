use flexi_logger::{DeferredNow, FileSpec, Logger, Record, WriteMode};
use green_languages::core::util::measurements_dir;
use green_languages::MeasureCommand;
use std::io::{Error, Write};
use clap::Parser;

fn custom_format(w: &mut dyn Write, now: &mut DeferredNow, record: &Record) -> Result<(), Error> {
    write!(
        w,
        "[{}] {:5} {}",
        now.format("%d-%m-%Y %H:%M:%S"),
        record.level(),
        record.args()
    )
}

fn configure_logger(output_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let spec = FileSpec::default()
        .directory(output_dir)
        .basename("measurements")
        .suppress_timestamp();
    Logger::try_with_str("info")?
        .log_to_file(spec)
        .write_mode(WriteMode::Direct)
        .format(custom_format)
        .append()
        .start()?;
    Ok(())
}

fn main() {
    let args = MeasureCommand::parse();
    let output_dir = args.output.clone().unwrap_or_else(measurements_dir);
    
    if let Err(err) = configure_logger(&output_dir) {
        eprintln!("Failed to configure logger: {}", err);
        std::process::exit(1);
    }
    
    if let Err(err) = MeasureCommand::handle(args) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

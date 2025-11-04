use flexi_logger::{DeferredNow, FileSpec, Logger, Record, WriteMode};
use green_languages::config::Config;
use green_languages::core::util::results_dir;
use green_languages::MeasureArgs;
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
    let spec = FileSpec::default()
        .directory(results_dir())
        .basename("gl")
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
    Config::global();
    if let Err(err) = configure_logger() {
        eprintln!("Failed to configure logger: {}", err);
        std::process::exit(1);
    }
    if let Err(err) = MeasureArgs::handle_args() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

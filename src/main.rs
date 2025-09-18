// use perf_event::{Builder, Group};
// use perf_event::events::Rapl;
// use std::{thread, time};

// fn measure() -> Result<f64, Box<dyn std::error::Error>> {
//     if !Rapl::PKG.is_available() {
//         return Err("RAPL energy measurements not available on this computer".into());
//     }

//     let mut group = Group::new()?;
//     let pkg = Rapl::PKG;
//     let mut counter = Builder::new()
//         .group(&mut group)
//         .kind(pkg)
//         .any_pid()
//         .one_cpu(0)
//         .build()?;
//     // let cores = Builder::new().group(&mut group).kind(Rapl::cores).build()?;

//     counter.enable()?;
//     thread::sleep(time::Duration::from_millis(100));
//     counter.disable()?;
//     let raw = counter.read()?;
//     let joules = pkg.to_joules(raw)?;
//     Ok(joules)
// }

// fn main() {
//     let result = measure();

//     match result {
//         Ok(val) => println!("Package Energy: {} J", val),
//         Err(err) => eprintln!("{}", err)
//     }
// }
mod commands;
mod config;
mod language;
mod scenario;

use crate::commands::cli::{Cli, Commands};
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Measure(args) => commands::measure::run(args),
    };

    if let Err(error) = result {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

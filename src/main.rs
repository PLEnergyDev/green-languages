use clap::Parser;
use green_languages::commands::cli::{Cli, Commands};
use green_languages::commands::measure::run;

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Measure(args) => run(args),
    };

    if let Err(error) = result {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

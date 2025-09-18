use crate::scenario::{BuildResult, Scenario};
use clap::Args;
use perf_event::events::Rapl;
use perf_event::Builder;
use std::path::PathBuf;

#[derive(Args)]
pub struct MeasureArgs {
    #[arg(short, long, default_value_t = 1)]
    iterations: u8,

    #[arg(short, long, default_value_t = 100)]
    frequency: u16,

    #[arg(short, long, default_value_t)]
    sleep: u8,

    #[arg(short, long, default_value_t = 100)]
    timeout: u8,

    #[arg(required = true, num_args = 1..)]
    scenarios: Vec<PathBuf>,
}

pub fn run(args: MeasureArgs) -> Result<(), Box<dyn std::error::Error>> {
    for scenario_file in &args.scenarios {
        let scenario = Scenario::try_from(scenario_file.as_path())?;
        match scenario.build() {
            Ok(BuildResult::Success { stdout, stderr }) => {
                println!("Ok!");
                if !stdout.is_empty() {
                    println!("   stdout: {}", stdout.trim());
                }
                if !stderr.is_empty() {
                    println!("   stderr: {}", stderr.trim());
                }
            }
            Ok(BuildResult::Failed {
                exit_code,
                stdout,
                stderr,
            }) => {
                match exit_code {
                    Some(val) => println!("Build failed (exit code: {})", val),
                    None => println!("Build failed"),
                }
                if !stdout.is_empty() {
                    println!("   stdout: {}", stdout.trim());
                }
                if !stderr.is_empty() {
                    println!("   stderr: {}", stderr.trim());
                }
                continue;
            }
            Ok(BuildResult::Skipped) => {}
            Err(err) => {
                continue;
            }
        }

        let pkg = Rapl::PKG;
        let mut counter = Builder::new()
            .kind(pkg)
            .any_pid()
            .one_cpu(0)
            .build()
            .expect("");
        counter.enable().expect("");
        let result = scenario.measure();
        counter.disable().expect("");
        let raw = counter.read().expect("");
        let joules = pkg.to_joules(raw).expect("");
        println!("Used {} Joules", joules);
    }
    Ok(())
}

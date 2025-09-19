use crate::scenario::result::ScenarioResult;
use crate::scenario::Scenario;
use crate::test::Test;
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
    let pkg = Rapl::PKG;
    let mut counter = Builder::new().kind(pkg).any_pid().one_cpu(0).build()?;

    for scenario_file in &args.scenarios {
        let scenario_path = scenario_file.as_path();
        let scenario = Scenario::try_from(scenario_path)?;
        let tests = Test::iterate_from_file(scenario_path)?;

        for (index, test_result) in tests.enumerate() {
            let mut test = match test_result? {
                mut t => {
                    if t.id.is_none() {
                        t.id = Some(index.to_string());
                    }
                    t
                }
            };

            match scenario.build_test(&mut test) {
                Ok(ScenarioResult::Success { stdout, stderr }) => {
                    println!("Ok!");
                    // if !stdout.is_empty() {
                    //     println!("   stdout: {}", stdout.trim());
                    // }
                    // if !stderr.is_empty() {
                    //     println!("   stderr: {}", stderr.trim());
                    // }
                }
                Ok(ScenarioResult::Failed {
                    exit_code,
                    stdout,
                    stderr,
                }) => {
                    match exit_code {
                        Some(val) => println!("Build failed (exit code: {})", val),
                        None => println!("Build failed"),
                    }
                    // if !stdout.is_empty() {
                    //     println!("   stdout: {}", stdout.trim());
                    // }
                    // if !stderr.is_empty() {
                    //     println!("   stderr: {}", stderr.trim());
                    // }
                    continue;
                }
                Ok(ScenarioResult::Skipped) => {
                    continue;
                }
                Err(err) => {
                    continue;
                }
            }
            counter.enable()?;
            scenario.execute_test(&test)?;
            counter.disable()?;
            let raw = counter.read()?;
            let joules = pkg.to_joules(raw)?;
            println!("Used {} Joules", joules);
        }
    }
    Ok(())
}

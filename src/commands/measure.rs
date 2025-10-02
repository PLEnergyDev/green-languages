use crate::scenario::result::ScenarioResult;
use crate::scenario::test::Test;
use crate::scenario::Scenario;
use clap::Args;
use csv::WriterBuilder;
use iterations::share::cleanup_shared_memory;
use iterations::signal::*;
use log::{error, info, warn};
use perf_event::events::Rapl;
use perf_event::{Builder, Counter, Group};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::OpenOptions;
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
    output_csv: PathBuf,
}

#[derive(Debug, Serialize)]
struct Measurement {
    scenario: String,
    test: String,
    pkg: Option<f64>,
    cores: Option<f64>,
    gpu: Option<f64>,
    dram: Option<f64>,
    psys: Option<f64>,
    iteration: usize,
    timestamp: i64,
}

impl Measurement {
    fn write_to_csv(&self, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let file_exists = output_path.exists();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(output_path)?;
        let mut wtr = WriterBuilder::new()
            .has_headers(!file_exists)
            .from_writer(file);

        wtr.serialize(self)?;
        wtr.flush()?;

        Ok(())
    }
}

struct Counters {
    group: Group,
    counters: HashMap<&'static str, Counter>,
}

impl Counters {
    fn new(args: &MeasureArgs) -> Result<Self, Box<dyn std::error::Error>> {
        let mut rapl_types = HashMap::new();

        if args.rapl_all {
            let all_domains = [
                ("pkg", Rapl::PKG),
                ("cores", Rapl::CORES),
                ("gpu", Rapl::GPU),
                ("dram", Rapl::DRAM),
                ("psys", Rapl::PSYS),
            ];

            for (name, rapl_type) in all_domains {
                if rapl_type.is_available() {
                    rapl_types.insert(name, rapl_type);
                }
            }
        } else {
            if args.rapl_pkg {
                rapl_types.insert("pkg", Rapl::PKG);
            }
            if args.rapl_cores {
                rapl_types.insert("cores", Rapl::CORES);
            }
            if args.rapl_gpu {
                rapl_types.insert("gpu", Rapl::GPU);
            }
            if args.rapl_dram {
                rapl_types.insert("dram", Rapl::DRAM);
            }
            if args.rapl_psys {
                rapl_types.insert("psys", Rapl::PSYS);
            }

            Self::check_availability(&rapl_types)?;
        }

        if rapl_types.is_empty() {
            rapl_types.insert("pkg", Rapl::PKG);
        }

        let first_rapl_type = *rapl_types.values().next().unwrap();
        let mut group = Group::rapl(first_rapl_type)?;

        let mut counters = HashMap::new();
        for (name, &rapl_type) in &rapl_types {
            let counter = Builder::new()
                .group(&mut group)
                .kind(rapl_type)
                .one_cpu(0)
                .any_pid_cloexec()
                .build()?;
            counters.insert(*name, counter);
        }

        Ok(Self { group, counters })
    }

    fn check_availability(
        rapl_types: &HashMap<&'static str, Rapl>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut unavailable_domains = Vec::new();

        for (domain_name, &rapl_type) in rapl_types {
            if !rapl_type.is_available() {
                unavailable_domains.push(*domain_name);
            }
        }

        if !unavailable_domains.is_empty() {
            return Err(format!(
                "RAPL domains not available: {}",
                unavailable_domains.join(", ")
            )
            .into());
        }

        Ok(())
    }

    fn read_measurements(
        &mut self,
        scenario_name: &str,
        test_id: &str,
        iteration: usize,
    ) -> Result<Measurement, Box<dyn std::error::Error>> {
        let counts = self.group.read()?;
        let timestamp = chrono::Utc::now().timestamp();

        let mut measurement = Measurement {
            scenario: scenario_name.to_string(),
            test: test_id.to_string(),
            pkg: None,
            cores: None,
            gpu: None,
            dram: None,
            psys: None,
            iteration: iteration,
            timestamp,
        };

        for (domain_name, counter) in &self.counters {
            let raw_value = counts[counter];
            match *domain_name {
                "pkg" => {
                    let j = Rapl::PKG.to_joules(raw_value)?;
                    measurement.pkg = Some(j);
                    j
                }
                "cores" => {
                    let j = Rapl::CORES.to_joules(raw_value)?;
                    measurement.cores = Some(j);
                    j
                }
                "gpu" => {
                    let j = Rapl::GPU.to_joules(raw_value)?;
                    measurement.gpu = Some(j);
                    j
                }
                "dram" => {
                    let j = Rapl::DRAM.to_joules(raw_value)?;
                    measurement.dram = Some(j);
                    j
                }
                "psys" => {
                    let j = Rapl::PSYS.to_joules(raw_value)?;
                    measurement.psys = Some(j);
                    j
                }
                _ => continue,
            };
        }

        Ok(measurement)
    }
}

pub fn run(args: MeasureArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut rapl = Counters::new(&args)?;
    init_shared_state()?;

    for scenario_file in &args.scenarios {
        let scenario_path = scenario_file.as_path();
        let scenario = Scenario::try_from(scenario_path)?;
        let tests = Test::iterate_from_file(scenario_path)?;
        let iterations: usize = args.iterations.into();

        for (index, test_result) in tests.enumerate() {
            let mut test = match test_result? {
                mut test => {
                    if test.id.is_none() {
                        test.id = Some(index.to_string());
                    }
                    test
                }
            };

            let test_id = test.id.as_ref().unwrap();
            let context = format!("[{}/{}]", scenario.name, test_id);

            match scenario.build_test(&mut test) {
                Ok(ScenarioResult::Success { out, err }) => {
                    info!("{} Build success", context);
                    if !out.trim().is_empty() {
                        info!("{} Build output:\n{}", context, out.trim());
                    }
                    if !err.trim().is_empty() {
                        warn!("{} Build stderr (warnings):\n{}", context, err.trim());
                    }
                }
                Ok(ScenarioResult::Failed { exit_code, err }) => {
                    error!("{} Build failed with exit code {}", context, exit_code);
                    if !err.trim().is_empty() {
                        error!("{} Build stderr:\n{}", context, err.trim());
                    }
                    continue;
                }
                Err(err) => {
                    error!("{} Build error: {}", context, err);
                    continue;
                }
            }

            set_iterations(iterations);
            let child = match scenario.exec_test_async(&test) {
                Ok(c) => c,
                Err(err) => {
                    error!("{} Failed to spawn process: {}", context, err);
                    continue;
                }
            };
            let mut measurements = Vec::new();

            for i in 0..iterations {
                wait_for_ready();
                rapl.group.reset()?;
                rapl.group.enable()?;
                signal_proceed();
                wait_for_measuring();
                wait_for_complete();
                rapl.group.disable()?;
                let measurement =
                    rapl.read_measurements(&scenario.name, &test.id.as_ref().unwrap(), i)?;
                measurements.push(measurement);
            }

            let output = match child.wait_with_output() {
                Ok(o) => o,
                Err(err) => {
                    error!("{} Failed to wait for process: {}", context, err);
                    continue;
                }
            };

            if output.status.success() {
                info!("{} Exec success", context);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    warn!("{} Exec stderr (warnings):\n{}", context, stderr.trim());
                }
            } else {
                let exit_code = output.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("{} Exec failed with exit code {}", context, exit_code);
                if !stderr.trim().is_empty() {
                    error!("{} Exec stderr:\n{}", context, stderr.trim());
                }
                continue;
            }

            match scenario.verify_test(&test, iterations) {
                Ok(ScenarioResult::Success { out, err }) => {
                    info!("{} Test success", context);
                    if !out.trim().is_empty() {
                        info!("{} Test output:\n{}", context, out.trim());
                    }
                    if !err.trim().is_empty() {
                        info!("{} Test stderr:\n{}", context, err.trim());
                    }

                    for measurement in measurements {
                        measurement.write_to_csv(&args.output_csv)?;
                    }
                }
                Ok(ScenarioResult::Failed { exit_code, err }) => {
                    error!("{} Test failed with exit code {}", context, exit_code);
                    if !err.trim().is_empty() {
                        error!("{} Test failure details:\n{}", context, err.trim());
                    }
                    continue;
                }
                Err(err) => {
                    error!("{} Test error: {}", context, err);
                    continue;
                }
            }
        }
        cleanup_shared_memory();
    }

    info!("Measurements saved to: {}", args.output_csv.display());
    Ok(())
}

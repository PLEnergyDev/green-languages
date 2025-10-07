use super::{MeasureArgs, Measurement};
use crate::core::Scenario;
use crate::core::ScenarioResult;
use crate::core::Test;
use csv::WriterBuilder;
use iterations::share::cleanup_shared_memory;
use iterations::signal::*;
use log::{error, info, warn};
use nix::sched::{sched_setaffinity, CpuSet};
use nix::unistd::Pid;
use perf_event::events::Rapl;
use perf_event::{Builder, Counter, Group};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::Child;
use std::thread;
use std::time::Duration;

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
        language_name: &str,
        scenario_name: &str,
        test_name: &str,
        iteration: usize,
        warmup: bool,
    ) -> Result<Measurement, Box<dyn std::error::Error>> {
        let counts = self.group.read()?;
        let timestamp = chrono::Utc::now().timestamp_micros();
        let mut measurement = Measurement {
            language: language_name.to_string(),
            scenario: scenario_name.to_string(),
            test: test_name.to_string(),
            warmup,
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

fn configure_process(child: &Child, args: &MeasureArgs, context: &str) {
    if let Some(cpu) = args.affinity {
        let pid = Pid::from_raw(child.id() as i32);
        let mut cpu_set = CpuSet::new();
        if let Err(e) = cpu_set.set(cpu) {
            warn!("{} Failed to configure CPU affinity: {}", context, e);
        } else {
            if let Err(e) = sched_setaffinity(pid, &cpu_set) {
                warn!("{} Failed to set CPU affinity: {}", context, e);
            }
        }
    }

    if let Some(nice_value) = args.niceness {
        unsafe {
            if libc::setpriority(libc::PRIO_PROCESS, child.id(), nice_value) != 0 {
                warn!("{} Failed to set process priority", context);
            }
        }
    }
}

fn measure_with_warmup(
    scenario: &Scenario,
    test: &Test,
    args: &MeasureArgs,
    rapl: &mut Counters,
    context: &str,
    iterations: usize,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = Vec::new();

    let child = scenario.exec_test_async(test)?;
    configure_process(&child, args, context);
    set_iterations(iterations);

    for i in 1..=iterations {
        wait_for_ready();

        rapl.group.reset()?;
        rapl.group.enable()?;

        signal_proceed();
        wait_for_measuring();
        wait_for_complete();

        rapl.group.disable()?;
        let measurement = rapl.read_measurements(
            &scenario.language.to_string(),
            &scenario.name,
            test.name.as_ref().unwrap(),
            i,
            true,
        )?;
        measurements.push(measurement);
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("{} Exec failed with exit code {}", context, exit_code);
        if !stderr.trim().is_empty() {
            error!("{} Exec stderr:\n{}", context, stderr.trim());
        }
        return Err(format!("Process exited with code {}", exit_code).into());
    }

    info!("{} Exec success", context);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        warn!("{} Exec stderr (warnings):\n{}", context, stderr.trim());
    }

    Ok(measurements)
}

fn measure_without_warmup(
    scenario: &Scenario,
    test: &Test,
    args: &MeasureArgs,
    rapl: &mut Counters,
    context: &str,
    iterations: usize,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = Vec::new();
    set_iterations(1);

    for i in 1..=iterations {
        let child = scenario.exec_test_async(test)?;
        configure_process(&child, args, context);

        wait_for_ready();

        rapl.group.reset()?;
        rapl.group.enable()?;

        signal_proceed();
        wait_for_measuring();
        wait_for_complete();

        rapl.group.disable()?;
        let measurement = rapl.read_measurements(
            &scenario.language.to_string(),
            &scenario.name,
            test.name.as_ref().unwrap(),
            i,
            false,
        )?;
        measurements.push(measurement);

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
                "{} Exec failed with exit code {} on iteration {}",
                context, exit_code, i
            );
            if !stderr.trim().is_empty() {
                error!("{} Exec stderr:\n{}", context, stderr.trim());
            }
            return Err(
                format!("Process exited with code {} on iteration {}", exit_code, i).into(),
            );
        }
    }

    info!("{} All iterations completed successfully", context);
    Ok(measurements)
}

pub fn run(args: MeasureArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut rapl = Counters::new(&args)?;

    for scenario_file in &args.scenarios {
        let scenario_path = scenario_file.as_path();
        let scenario = Scenario::try_from(scenario_path)?;
        let tests = Test::iterate_from_file(scenario_path)?;
        let iterations: usize = args.iterations.into();

        init_shared_state()?;

        for (index, test_result) in tests.enumerate() {
            let mut test = match test_result? {
                mut t => {
                    if t.name.is_none() {
                        t.name = Some(index.to_string());
                    }
                    t
                }
            };

            let test_name = test.name.as_ref().unwrap();
            let context = format!("[{}/{}]", scenario.name, test_name);

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
                Ok(ScenarioResult::Failed {
                    exit_code,
                    out,
                    err,
                }) => {
                    error!("{} Build failed with exit code {}", context, exit_code);
                    if !err.trim().is_empty() {
                        error!("{} Build stderr:\n{}", context, err.trim());
                    }
                    if !out.trim().is_empty() {
                        error!("{} Build stdout:\n{}", context, out.trim());
                    }
                    continue;
                }
                Err(err) => {
                    error!("{} Build error: {}", context, err);
                    continue;
                }
            }

            let measurements = if args.warmup {
                match measure_with_warmup(&scenario, &test, &args, &mut rapl, &context, iterations)
                {
                    Ok(m) => m,
                    Err(err) => {
                        error!("{} Measurement error: {}", context, err);
                        continue;
                    }
                }
            } else {
                match measure_without_warmup(
                    &scenario, &test, &args, &mut rapl, &context, iterations,
                ) {
                    Ok(m) => m,
                    Err(err) => {
                        error!("{} Measurement error: {}", context, err);
                        continue;
                    }
                }
            };

            let verify_iterations = if args.warmup { iterations } else { 1 };
            match scenario.verify_test(&test, verify_iterations) {
                Ok(ScenarioResult::Success { out, err }) => {
                    info!("{} Test success", context);
                    if !out.trim().is_empty() {
                        info!("{} Test output:\n{}", context, out.trim());
                    }
                    if !err.trim().is_empty() {
                        info!("{} Test stderr:\n{}", context, err.trim());
                    }

                    for measurement in measurements {
                        measurement.write_to_csv(&args.output)?;
                    }
                    info!("Measurements saved: {}", args.output.display());
                }
                Ok(ScenarioResult::Failed {
                    exit_code,
                    out,
                    err,
                }) => {
                    error!("{} Test failed with exit code {}", context, exit_code);
                    if !err.trim().is_empty() {
                        error!("{} Test failure details:\n{}", context, err.trim());
                    }
                    if !out.trim().is_empty() {
                        error!("{} Test failure details:\n{}", context, out.trim());
                    }
                    continue;
                }
                Err(err) => {
                    error!("{} Test error: {}", context, err);
                    continue;
                }
            }

            if let Some(secs) = args.sleep {
                info!("{} Sleeping for {} seconds", context, secs);
                thread::sleep(Duration::from_secs(secs as u64));
            }
        }
        cleanup_shared_memory();
    }

    Ok(())
}

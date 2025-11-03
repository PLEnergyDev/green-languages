use super::{MeasureArgs, Measurement};
use crate::core::util::results_dir;
use crate::core::{MeasurementMode, Scenario, ScenarioResult, Test};
use csv::WriterBuilder;
use log::{error, info, warn};
use measurements::share::cleanup_shared_memory;
use measurements::signal::*;
use nix::sched::{sched_setaffinity, CpuSet};
use nix::unistd::Pid;
use perf_event::events::{Cache, CacheOp, CacheResult, Hardware, Rapl, WhichCache};
use perf_event::{Builder, Counter, Group};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

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

struct RaplCounters {
    group: Group,
    counters: HashMap<&'static str, Counter>,
}

impl RaplCounters {
    fn new(args: &MeasureArgs) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let has_rapl = args.rapl_pkg
            || args.rapl_cores
            || args.rapl_gpu
            || args.rapl_dram
            || args.rapl_psys
            || args.rapl_all;

        if !has_rapl {
            return Ok(None);
        }

        let mut types = HashMap::new();

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
                    types.insert(name, rapl_type);
                }
            }
        } else {
            if args.rapl_pkg {
                types.insert("pkg", Rapl::PKG);
            }
            if args.rapl_cores {
                types.insert("cores", Rapl::CORES);
            }
            if args.rapl_gpu {
                types.insert("gpu", Rapl::GPU);
            }
            if args.rapl_dram {
                types.insert("dram", Rapl::DRAM);
            }
            if args.rapl_psys {
                types.insert("psys", Rapl::PSYS);
            }

            Self::check_rapl_availability(&types)?;
        }

        if types.is_empty() {
            types.insert("pkg", Rapl::PKG);
        }

        let first_rapl_type = *types.values().next().unwrap();
        let mut group = Group::rapl(first_rapl_type)?;
        let mut counters = HashMap::new();

        for (n, &t) in &types {
            let counter = Builder::new()
                .group(&mut group)
                .kind(t)
                .one_cpu(0)
                .any_pid_cloexec()
                .build()?;
            counters.insert(*n, counter);
        }

        Ok(Some(Self { group, counters }))
    }

    fn check_rapl_availability(
        types: &HashMap<&'static str, Rapl>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut unavailable_domains = Vec::new();

        for (n, &t) in types {
            if !t.is_available() {
                unavailable_domains.push(*n);
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

    fn read_into_measurement(
        &mut self,
        measurement: &mut Measurement,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let counts = self.group.read()?;

        for (domain_name, counter) in &self.counters {
            let raw_value = counts[counter];
            match *domain_name {
                "pkg" => {
                    measurement.pkg = Some(Rapl::PKG.to_joules(raw_value)?);
                }
                "cores" => {
                    measurement.cores = Some(Rapl::CORES.to_joules(raw_value)?);
                }
                "gpu" => {
                    measurement.gpu = Some(Rapl::GPU.to_joules(raw_value)?);
                }
                "dram" => {
                    measurement.dram = Some(Rapl::DRAM.to_joules(raw_value)?);
                }
                "psys" => {
                    measurement.psys = Some(Rapl::PSYS.to_joules(raw_value)?);
                }
                _ => continue,
            }
        }

        Ok(())
    }
}

struct HardwareCounters {
    group: Group,
    counters: HashMap<&'static str, Counter>,
}

impl HardwareCounters {
    fn new(args: &MeasureArgs) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let has_hardware =
            args.hw_cycles || args.hw_cache_misses || args.hw_branch_misses || args.hw_all;

        if !has_hardware {
            return Ok(None);
        }

        let mut group = Group::new()?;
        let mut counters = HashMap::new();

        if args.hw_all || args.hw_cycles {
            let counter = Builder::new()
                .group(&mut group)
                .kind(Hardware::CPU_CYCLES)
                .build()?;
            counters.insert("cycles", counter);
        }

        if args.hw_all || args.hw_cache_misses {
            const L1D_MISS: Cache = Cache {
                which: WhichCache::L1D,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            };
            const L1I_MISS: Cache = Cache {
                which: WhichCache::L1I,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            };
            const LLC_MISS: Cache = Cache {
                which: WhichCache::LL,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            };

            let l1d_counter = Builder::new().group(&mut group).kind(L1D_MISS).build()?;
            counters.insert("l1d_misses", l1d_counter);

            let l1i_counter = Builder::new().group(&mut group).kind(L1I_MISS).build()?;
            counters.insert("l1i_misses", l1i_counter);

            let llc_counter = Builder::new().group(&mut group).kind(LLC_MISS).build()?;
            counters.insert("llc_misses", llc_counter);
        }

        if args.hw_all || args.hw_branch_misses {
            let counter = Builder::new()
                .group(&mut group)
                .kind(Hardware::BRANCH_MISSES)
                .build()?;
            counters.insert("branch_misses", counter);
        }

        Ok(Some(Self { group, counters }))
    }

    fn read_into_measurement(
        &mut self,
        measurement: &mut Measurement,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let counts = self.group.read()?;

        for (event_name, counter) in &self.counters {
            let raw_value = counts[counter];
            match *event_name {
                "cycles" => {
                    measurement.cycles = Some(raw_value);
                }
                "l1d_misses" => {
                    measurement.l1d_misses = Some(raw_value);
                }
                "l1i_misses" => {
                    measurement.l1i_misses = Some(raw_value);
                }
                "llc_misses" => {
                    measurement.llc_misses = Some(raw_value);
                }
                "branch_misses" => {
                    measurement.branch_misses = Some(raw_value);
                }
                _ => continue,
            }
        }

        Ok(())
    }
}

struct Counters {
    rapl: Option<RaplCounters>,
    hardware: Option<HardwareCounters>,
    measure_time: bool,
    start_time: Option<Instant>,
}

impl Counters {
    fn new(args: &MeasureArgs) -> Result<Self, Box<dyn std::error::Error>> {
        let rapl = RaplCounters::new(args)?;
        let hardware = HardwareCounters::new(args)?;
        let measure_time = args.time;

        if rapl.is_none() && hardware.is_none() && !measure_time {
            return Err("No events specified. Use --rapl-*, --hw-*, or --time flags".into());
        }

        Ok(Self {
            rapl,
            hardware,
            measure_time,
            start_time: None,
        })
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut rapl) = self.rapl {
            rapl.group.reset()?;
        }
        if let Some(ref mut hardware) = self.hardware {
            hardware.group.reset()?;
        }
        Ok(())
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.measure_time {
            self.start_time = Some(Instant::now());
        }
        if let Some(ref mut hardware) = self.hardware {
            hardware.group.enable()?;
        }
        if let Some(ref mut rapl) = self.rapl {
            rapl.group.enable()?;
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut rapl) = self.rapl {
            rapl.group.disable()?;
        }
        if let Some(ref mut hardware) = self.hardware {
            hardware.group.disable()?;
        }
        Ok(())
    }

    fn read_measurements(
        &mut self,
        language_name: &str,
        scenario_name: &str,
        test_name: &str,
        iteration: usize,
        mode: MeasurementMode,
    ) -> Result<Measurement, Box<dyn std::error::Error>> {
        let ended = chrono::Utc::now().timestamp_micros();
        let mut measurement = Measurement {
            language: language_name.to_string(),
            scenario: scenario_name.to_string(),
            test: test_name.to_string(),
            mode,
            iteration,
            time: None,
            pkg: None,
            cores: None,
            gpu: None,
            dram: None,
            psys: None,
            cycles: None,
            l1d_misses: None,
            l1i_misses: None,
            llc_misses: None,
            branch_misses: None,
            ended,
        };

        if let Some(ref mut rapl) = self.rapl {
            rapl.read_into_measurement(&mut measurement)?;
        }
        if let Some(start) = self.start_time {
            measurement.time = Some(start.elapsed().as_nanos() as u64);
        }
        if let Some(ref mut hardware) = self.hardware {
            hardware.read_into_measurement(&mut measurement)?;
        }
        Ok(measurement)
    }
}

fn configure_process(
    child: &Child,
    affinity: &Option<Vec<usize>>,
    niceness: Option<i32>,
    context: &str,
) {
    if let Some(cpus) = affinity {
        let pid = Pid::from_raw(child.id() as i32);
        let mut cpu_set = CpuSet::new();

        for &c in cpus {
            if let Err(e) = cpu_set.set(c) {
                warn!("{} Failed to add CPU {} to affinity set: {}", context, c, e);
            }
        }
        if let Err(e) = sched_setaffinity(pid, &cpu_set) {
            warn!("{} Failed to set CPU affinity: {}", context, e);
        }
    }

    if let Some(nice_value) = niceness {
        unsafe {
            if libc::setpriority(libc::PRIO_PROCESS, child.id(), nice_value) != 0 {
                warn!("{} Failed to set process priority", context);
            }
        }
    }
}

fn process_single_test(
    scenario: &mut Scenario,
    mut test: Test,
    index: usize,
    counters: &mut Counters,
    args: &MeasureArgs,
    iterations: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = test.name.as_ref().unwrap();
    let context = format!("[{}/{}]", scenario.name, test_name);
    let affinity = test.affinity.clone().or(scenario.affinity.clone());
    let niceness = test.niceness.or(scenario.niceness);
    let measurement_mode = test
        .measurement_mode
        .or(scenario.measurement_mode)
        .unwrap_or(MeasurementMode::Process);

    info!("{} Build started", context);
    match scenario.build_test(&mut test, index) {
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
            return Ok(());
        }
        Err(err) => {
            error!("{} Build error: {}", context, err);
            return Ok(());
        }
    }

    info!("{} Measurement start", context);
    let measurements = match measurement_mode {
        MeasurementMode::Internal => match measure_internal(
            &scenario, &test, counters, &context, iterations, &affinity, niceness,
        ) {
            Ok(m) => m,
            Err(err) => {
                error!("{} Measurement error: {}", context, err);
                return Ok(());
            }
        },
        MeasurementMode::External => match measure_external(
            &scenario, &test, counters, &context, iterations, &affinity, niceness,
        ) {
            Ok(m) => m,
            Err(err) => {
                error!("{} Measurement error: {}", context, err);
                return Ok(());
            }
        },
        MeasurementMode::Process => match measure_process(
            &scenario, &test, counters, &context, iterations, &affinity, niceness,
        ) {
            Ok(m) => m,
            Err(err) => {
                error!("{} Measurement error: {}", context, err);
                return Ok(());
            }
        },
    };

    let test_expected_stdout_path = scenario.test_expected_stdout_path(&test);
    let scenario_expected_stdout_path = scenario.scenario_expected_stdout_path();
    let should_verify =
        test_expected_stdout_path.exists() || scenario_expected_stdout_path.exists();

    if should_verify {
        info!("{} Test started", context);
        let verify_iterations = match measurement_mode {
            MeasurementMode::Internal => iterations,
            _ => 1,
        };
        match scenario.verify_test(&test, verify_iterations) {
            Ok(ScenarioResult::Success { out, err }) => {
                info!("{} Test success", context);
                if !out.trim().is_empty() {
                    info!("{} Test output:\n{}", context, out.trim());
                }
                if !err.trim().is_empty() {
                    info!("{} Test stderr:\n{}", context, err.trim());
                }
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
                return Ok(());
            }
            Err(err) => {
                error!("{} Test error: {}", context, err);
                return Ok(());
            }
        }
    }

    let output_path = if let Some(ref user_path) = args.output {
        if let Some(parent) = user_path.parent() {
            fs::create_dir_all(parent)?;
        }
        user_path.clone()
    } else {
        results_dir().join("results.csv")
    };

    for measurement in measurements {
        measurement.write_to_csv(&output_path)?;
    }
    info!("Measurements saved: {}", output_path.display());

    if args.sleep > 0 {
        info!("{} Sleeping for {} seconds", context, args.sleep);
        thread::sleep(Duration::from_secs(args.sleep as u64));
    }

    Ok(())
}

pub fn run(args: MeasureArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut counters = Counters::new(&args)?;

    for scenario_file in &args.scenarios {
        let scenario_path = scenario_file.as_path();
        let mut scenario = Scenario::try_from(scenario_path)?;
        let mut tests = Test::iterate_from_file(scenario_path)?.peekable();
        let iterations: usize = args.iterations.into();

        let scenario_dir = scenario.scenario_dir();
        if scenario_dir.exists() {
            fs::remove_dir_all(&scenario_dir)?;
        }

        init_shared_state()?;

        if tests.peek().is_none() {
            process_single_test(
                &mut scenario,
                Test::default(),
                0,
                &mut counters,
                &args,
                iterations,
            )?;
        } else {
            for (index, test_result) in tests.enumerate() {
                let test = test_result?;
                process_single_test(&mut scenario, test, index, &mut counters, &args, iterations)?;
            }
        }

        cleanup_shared_memory();
    }

    Ok(())
}

fn measure_internal(
    scenario: &Scenario,
    test: &Test,
    counters: &mut Counters,
    context: &str,
    iterations: usize,
    affinity: &Option<Vec<usize>>,
    niceness: Option<i32>,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = Vec::new();
    set_iterations(iterations);

    let child = scenario.exec_test_async(test, MeasurementMode::Internal)?;
    configure_process(&child, affinity, niceness, context);

    for iteration in 1..=iterations {
        wait_for_ready();

        counters.reset()?;
        counters.enable()?;

        signal_proceed();
        wait_for_complete();

        counters.disable()?;

        let measurement = counters.read_measurements(
            &scenario.language.to_string(),
            &scenario.name,
            test.name.as_ref().unwrap(),
            iteration,
            MeasurementMode::Internal,
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

fn measure_external(
    scenario: &Scenario,
    test: &Test,
    counters: &mut Counters,
    context: &str,
    iterations: usize,
    affinity: &Option<Vec<usize>>,
    niceness: Option<i32>,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = Vec::new();

    for iteration in 1..=iterations {
        set_iterations(1);

        let child = scenario.exec_test_async(test, MeasurementMode::External)?;
        configure_process(&child, affinity, niceness, context);

        wait_for_ready();

        counters.reset()?;
        counters.enable()?;

        signal_proceed();
        wait_for_complete();

        counters.disable()?;

        let measurement = counters.read_measurements(
            &scenario.language.to_string(),
            &scenario.name,
            test.name.as_ref().unwrap(),
            iteration,
            MeasurementMode::External,
        )?;

        let output = child.wait_with_output()?;
        finalize_measurement(output, context, iteration, measurement, &mut measurements)?;
    }

    info!("{} All iterations completed successfully", context);
    Ok(measurements)
}

fn measure_process(
    scenario: &Scenario,
    test: &Test,
    counters: &mut Counters,
    context: &str,
    iterations: usize,
    affinity: &Option<Vec<usize>>,
    niceness: Option<i32>,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = Vec::new();

    for iteration in 1..=iterations {
        let child = scenario.exec_test_async(test, MeasurementMode::Process)?;
        configure_process(&child, affinity, niceness, context);

        counters.reset()?;
        counters.enable()?;

        let output = child.wait_with_output()?;

        counters.disable()?;

        let measurement = counters.read_measurements(
            &scenario.language.to_string(),
            &scenario.name,
            test.name.as_ref().unwrap(),
            iteration,
            MeasurementMode::Process,
        )?;

        finalize_measurement(output, context, iteration, measurement, &mut measurements)?;
    }

    info!("{} All iterations completed successfully", context);
    Ok(measurements)
}

fn finalize_measurement(
    output: std::process::Output,
    context: &str,
    iteration: usize,
    measurement: Measurement,
    measurements: &mut Vec<Measurement>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            "{} Exec failed with exit code {} on iteration {}",
            context, exit_code, iteration
        );
        if !stderr.trim().is_empty() {
            error!("{} Exec stderr:\n{}", context, stderr.trim());
        }
        return Err(format!(
            "Process exited with code {} on iteration {}",
            exit_code, iteration
        )
        .into());
    }

    measurements.push(measurement);
    Ok(())
}

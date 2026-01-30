use crate::core::util::measurements_dir;
use crate::core::{MeasurementMode, Scenario, ScenarioResult, Test};
use crate::{MeasureCommand, Measurement};
use csv::WriterBuilder;
use log::{error, info, warn};
use signals::signal;
use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Dynamic, Hardware, Software};
use perf_event::{Builder, Counter, Group};
use perf_event_data::ReadFormat;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Output;
use std::time::Instant;

trait Bundle {
    fn new(affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>>;
}

struct BundleConfig {
    rapl: bool,
    misses: bool,
    cstates: bool,
    cycles: bool,
}

impl BundleConfig {
    fn create_bundles(
        &self,
        affinity: &Option<Vec<usize>>,
    ) -> Result<Vec<Box<dyn Bundle>>, Box<dyn std::error::Error>> {
        let mut bundles: Vec<Box<dyn Bundle>> = vec![];

        bundles.push(Box::new(TimeBundle::new(affinity)?));

        if self.rapl {
            bundles.push(Box::new(RaplBundle::new(affinity)?));
        }
        if self.misses {
            bundles.push(Box::new(MissesBundle::new(affinity)?));
        }
        if self.cstates {
            bundles.push(Box::new(CStateBundle::new(affinity)?));
        }
        if self.cycles {
            bundles.push(Box::new(CyclesBundle::new(affinity)?));
        }

        Ok(bundles)
    }
}

struct RaplCounter {
    counter: Counter,
    scale: f64,
}

struct RaplBundle {
    group: Group,
    counters: HashMap<&'static str, RaplCounter>,
}

struct MissesBundle {
    counters: HashMap<&'static str, Vec<Counter>>,
}

struct CStateBundle {
    counters: HashMap<String, Counter>,
}

struct CyclesBundle {
    counters: Vec<Counter>,
}

struct TimeBundle {
    start_time: Option<Instant>,
}

impl Bundle for TimeBundle {
    fn new(_affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { start_time: None })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_time = Some(Instant::now());
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_time = None;
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut measurements = HashMap::new();
        if let Some(start) = self.start_time {
            let elapsed_micros = start.elapsed().as_micros() as u64;
            measurements.insert("time".to_string(), elapsed_micros.to_string());
        }
        Ok(measurements)
    }
}

impl Bundle for RaplBundle {
    fn new(_affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>> {
        let rapl_events = vec![
            "energy-pkg",
            "energy-cores",
            "energy-gpu",
            "energy-psys",
            "energy-ram",
        ];
        let mut group = Builder::new(Software::DUMMY)
            .read_format(ReadFormat::GROUP | ReadFormat::TOTAL_TIME_RUNNING)
            .one_cpu(0)
            .any_pid()
            .exclude_hv(false)
            .exclude_kernel(false)
            .build_group()?;
        let mut counters = HashMap::new();

        for event_name in rapl_events {
            if let Ok(mut builder) = Dynamic::builder("power") {
                if builder.event(event_name).is_ok() {
                    if let Ok(Some(scale)) = builder.scale() {
                        if let Ok(built_event) = builder.build() {
                            if let Ok(counter) = Builder::new(built_event)
                                .one_cpu(0)
                                .any_pid()
                                .exclude_hv(false)
                                .exclude_kernel(false)
                                .build_with_group(&mut group)
                            {
                                counters.insert(event_name, RaplCounter { counter, scale });
                            }
                        }
                    }
                }
            }
        }

        Ok(Self { group, counters })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.enable()?;
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.disable()?;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.reset()?;
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut measurements = HashMap::new();
        for (name, rapl_counter) in &mut self.counters {
            let raw = rapl_counter.counter.read()?;
            let scaled = raw as f64 * rapl_counter.scale;
            measurements.insert(name.to_string(), format!("{:.3}", scaled));
        }
        Ok(measurements)
    }
}

impl Bundle for MissesBundle {
    fn new(affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut counters = HashMap::new();

        let cpus = if let Some(affinity_cpus) = affinity {
            affinity_cpus.clone()
        } else {
            (0..num_cpus::get()).collect()
        };

        const L1D_MISS: Cache = Cache {
            which: CacheId::L1D,
            operation: CacheOp::READ,
            result: CacheResult::MISS,
        };
        const L1I_MISS: Cache = Cache {
            which: CacheId::L1I,
            operation: CacheOp::READ,
            result: CacheResult::MISS,
        };
        const LLC_MISS: Cache = Cache {
            which: CacheId::LL,
            operation: CacheOp::READ,
            result: CacheResult::MISS,
        };

        enum EventType {
            L1dMiss,
            L1iMiss,
            LlcMiss,
            BranchMiss,
        }

        impl EventType {
            fn name(&self) -> &'static str {
                match self {
                    EventType::L1dMiss => "l1d_misses",
                    EventType::L1iMiss => "l1i_misses",
                    EventType::LlcMiss => "llc_misses",
                    EventType::BranchMiss => "branch_misses",
                }
            }

            fn build_counter(&self, cpu: usize) -> Result<Counter, std::io::Error> {
                let mut builder = match self {
                    EventType::L1dMiss => Builder::new(L1D_MISS),
                    EventType::L1iMiss => Builder::new(L1I_MISS),
                    EventType::LlcMiss => Builder::new(LLC_MISS),
                    EventType::BranchMiss => Builder::new(Hardware::BRANCH_MISSES),
                };

                builder
                    .one_cpu(cpu)
                    .any_pid()
                    .exclude_kernel(false)
                    .exclude_hv(false)
                    .build()
            }
        }

        let events = [
            EventType::L1dMiss,
            EventType::L1iMiss,
            EventType::LlcMiss,
            EventType::BranchMiss,
        ];

        for event in &events {
            let cpu_counters: Vec<Counter> = cpus
                .iter()
                .filter_map(|&cpu| event.build_counter(cpu).ok())
                .collect();

            if !cpu_counters.is_empty() {
                counters.insert(event.name(), cpu_counters);
            }
        }

        Ok(Self { counters })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for cpu_counters in self.counters.values_mut() {
            for counter in cpu_counters {
                counter.enable()?;
            }
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for cpu_counters in self.counters.values_mut() {
            for counter in cpu_counters {
                counter.disable()?;
            }
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for cpu_counters in self.counters.values_mut() {
            for counter in cpu_counters {
                counter.reset()?;
            }
        }
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut measurements = HashMap::new();

        for (name, cpu_counters) in &mut self.counters {
            let mut total: u64 = 0;
            for counter in cpu_counters {
                total += counter.read()?;
            }
            measurements.insert(name.to_string(), total.to_string());
        }

        Ok(measurements)
    }
}

impl Bundle for CStateBundle {
    fn new(affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>> {
        let core_events = vec![
            "c1-residency",
            "c3-residency",
            "c6-residency",
            "c7-residency",
        ];
        let pkg_events = vec![
            "c2-residency",
            "c3-residency",
            "c6-residency",
            "c8-residency",
            "c10-residency",
        ];
        let mut counters = HashMap::new();

        let cpus = if let Some(affinity_cpus) = affinity {
            affinity_cpus.clone()
        } else {
            (0..num_cpus::get_physical()).collect()
        };

        for event_name in core_events {
            for &cpu in &cpus {
                if let Ok(mut builder) = Dynamic::builder("cstate_core") {
                    if builder.event(event_name).is_ok() {
                        if let Ok(built_event) = builder.build() {
                            if let Ok(counter) = Builder::new(built_event)
                                .one_cpu(cpu)
                                .any_pid()
                                .exclude_kernel(false)
                                .exclude_hv(false)
                                .build()
                            {
                                let key = format!("cstate_core/{}_{}", event_name, cpu);
                                counters.insert(key, counter);
                            }
                        }
                    }
                }
            }
        }

        for event_name in pkg_events {
            if let Ok(mut builder) = Dynamic::builder("cstate_pkg") {
                if builder.event(event_name).is_ok() {
                    if let Ok(built_event) = builder.build() {
                        if let Ok(counter) = Builder::new(built_event)
                            .one_cpu(0)
                            .any_pid()
                            .exclude_kernel(false)
                            .exclude_hv(false)
                            .build()
                        {
                            let key = format!("cstate_pkg/{}", event_name);
                            counters.insert(key, counter);
                        }
                    }
                }
            }
        }

        Ok(Self { counters })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.enable()?;
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.disable()?;
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.reset()?;
        }
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut aggregated: HashMap<String, u64> = HashMap::new();

        for (name, counter) in &mut self.counters {
            let value = counter.read()?;

            let event_name = match name.as_str() {
                s if s.starts_with("cstate_core/") => {
                    let core_event = s
                        .strip_prefix("cstate_core/")
                        .and_then(|s| s.split('_').next())
                        .unwrap_or(s);
                    format!("cstate_core/{}", core_event)
                }
                s if s.starts_with("cstate_pkg/") => s.to_string(),
                _ => name.clone(),
            };

            *aggregated.entry(event_name).or_insert(0) += value;
        }

        let mut measurements = HashMap::new();
        for (name, value) in aggregated {
            measurements.insert(name, value.to_string());
        }
        Ok(measurements)
    }
}

impl Bundle for CyclesBundle {
    fn new(affinity: &Option<Vec<usize>>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut counters = Vec::new();

        let cpus = if let Some(affinity_cpus) = affinity {
            affinity_cpus.clone()
        } else {
            (0..num_cpus::get()).collect()
        };

        for &cpu in &cpus {
            if let Ok(counter) = Builder::new(Hardware::CPU_CYCLES)
                .one_cpu(cpu)
                .any_pid()
                .exclude_kernel(false)
                .exclude_hv(false)
                .build()
            {
                counters.push(counter);
            }
        }

        Ok(Self { counters })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in &mut self.counters {
            counter.enable()?;
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in &mut self.counters {
            counter.disable()?;
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in &mut self.counters {
            counter.reset()?;
        }
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut measurements = HashMap::new();

        let mut total_cycles: u64 = 0;
        for counter in &mut self.counters {
            total_cycles += counter.read()?;
        }

        measurements.insert("cycles".to_string(), total_cycles.to_string());
        Ok(measurements)
    }
}

impl Measurement {
    fn write_to_csv(&self, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = output_dir.join("measurements.csv");
        let file_exists = csv_path.exists();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(csv_path)?;
        let mut wtr = WriterBuilder::new()
            .has_headers(!file_exists)
            .from_writer(file);

        wtr.serialize(self)?;
        wtr.flush()?;

        Ok(())
    }

    fn new(
        scenario: &Scenario,
        test: &Test,
        mode: MeasurementMode,
        iteration: usize,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
    ) -> Self {
        Self {
            language: scenario.language.to_string(),
            scenario: scenario.name.clone(),
            test: test.name.as_ref().unwrap().clone(),
            nice: niceness,
            affinity: affinity.as_ref().map(|cpus| {
                cpus.iter()
                    .map(|cpu| cpu.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }),
            mode,
            iteration,
            time: None,
            pkg: None,
            cores: None,
            gpu: None,
            ram: None,
            psys: None,
            cycles: None,
            l1d_misses: None,
            l1i_misses: None,
            llc_misses: None,
            branch_misses: None,
            c1_core_residency: None,
            c3_core_residency: None,
            c6_core_residency: None,
            c7_core_residency: None,
            c2_pkg_residency: None,
            c3_pkg_residency: None,
            c6_pkg_residency: None,
            c8_pkg_residency: None,
            c10_pkg_residency: None,
            ended: chrono::Utc::now().timestamp_micros(),
        }
    }
}

impl MeasureCommand {
    pub fn handle(args: Self) -> Result<(), Box<dyn std::error::Error>> {
        let bundle_config = BundleConfig {
            rapl: args.rapl,
            misses: args.misses,
            cstates: args.cstates,
            cycles: args.cycles,
        };

        let output_dir = args.output.clone().unwrap_or_else(measurements_dir);
        std::fs::create_dir_all(&output_dir)?;

        for scenario_path_str in &args.scenarios {
            let scenario_path = scenario_path_str.as_path();
            let mut scenario = Scenario::try_from(scenario_path)?;
            let tests = Test::iterate_from_file(scenario_path)?.peekable();
            let iterations: usize = args.iterations.into();
            let mut has_tests = false;

            let scenario_dir = scenario.scenario_dir(&output_dir);
            if scenario_dir.exists() {
                fs::remove_dir_all(&scenario_dir)?;
            }

            for (index, test_result) in tests.enumerate() {
                has_tests = true;
                let mut test = test_result?;

                if test.name.is_none() {
                    test.name = Some((index + 1).to_string());
                }

                if let Err(err) = Self::process_test(
                    &mut scenario,
                    test,
                    &bundle_config,
                    0,
                    iterations,
                    args.sleep,
                    &output_dir,
                ) {
                    error!("{}", err);
                }
            }

            if !has_tests {
                let mut test = Test::default();
                test.name = Some("1".to_string());

                if let Err(err) = Self::process_test(
                    &mut scenario,
                    test,
                    &bundle_config,
                    0,
                    iterations,
                    args.sleep,
                    &output_dir,
                ) {
                    error!("{}", err);
                }
            }
        }

        Ok(())
    }

    fn verify_iteration(
        scenario: &mut Scenario,
        test: &Test,
        verify_iterations: usize,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let test_expected_stdout_path = scenario.test_expected_stdout_path(test, output_dir);
        let scenario_expected_stdout_path = scenario.scenario_expected_stdout_path(output_dir);

        if !test_expected_stdout_path.exists() && !scenario_expected_stdout_path.exists() {
            return Ok(());
        }

        match scenario.verify_test(&test, verify_iterations, output_dir) {
            Ok(ScenarioResult::Success { out, err }) => {
                if !out.trim().is_empty() {
                    info!("    Verification output:\n{}", out.trim());
                }
                if !err.trim().is_empty() {
                    info!("    Verification stderr:\n{}", err.trim());
                }
                Ok(())
            }
            Ok(ScenarioResult::Failed {
                exit_code,
                out,
                err,
            }) => {
                if !err.trim().is_empty() {
                    error!("    Verification failed:\n{}", err.trim());
                }
                if !out.trim().is_empty() {
                    error!("    Verification output:\n{}", out.trim());
                }
                Err(format!("Verification failed with exit code {}", exit_code).into())
            }
            Err(err) => Err(format!("Verification error: {}", err).into()),
        }
    }

    fn process_test(
        scenario: &mut Scenario,
        mut test: Test,
        bundle_config: &BundleConfig,
        iter_index: usize,
        iterations: usize,
        sleep: u8,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let test_name = test.name.as_ref().unwrap();
        let mode = test.mode
            .or(scenario.mode)
            .unwrap_or(MeasurementMode::Process);
        let affinity = test.affinity.clone().or(scenario.affinity.clone());
        let niceness = test.niceness.or(scenario.niceness);
        let affinity_str = affinity
            .as_ref()
            .map(|a| a.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(","))
            .unwrap_or_else(|| "-".to_string());
        let niceness_str = niceness
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let context = format!(
            "[{} | {} | {} | {} | {}@{}]",
            scenario.language, scenario.name, test_name, mode, niceness_str, affinity_str
        );

        info!("{}", context);
        info!("  Build started");
        match scenario.build_test(&mut test, iter_index, output_dir) {
            Ok(ScenarioResult::Success { out, err }) => {
                info!("  Build success");
                if !out.trim().is_empty() {
                    info!("  Build output:\n{}", out.trim());
                }
                if !err.trim().is_empty() {
                    warn!("  Build warnings:\n{}", err.trim());
                }
            }
            Ok(ScenarioResult::Failed {
                exit_code,
                out,
                err,
            }) => {
                if !err.trim().is_empty() {
                    error!("  Build stderr:\n{}", err.trim());
                }
                if !out.trim().is_empty() {
                    error!("  Build stdout:\n{}", out.trim());
                }
                return Err(format!("Build failed with exit code {}", exit_code).into());
            }
            Err(err) => {
                return Err(format!("Build error: {}", err).into());
            }
        }

        let mut bundles = bundle_config.create_bundles(&affinity)?;

        info!("  Test started ({} iterations)", iterations);
        match mode {
            MeasurementMode::Internal => Self::measure_internal(
                scenario,
                &test,
                iterations,
                sleep,
                &affinity,
                niceness,
                output_dir,
                &mut bundles,
            )?,
            MeasurementMode::External => Self::measure_external(
                scenario,
                &test,
                iterations,
                sleep,
                &affinity,
                niceness,
                output_dir,
                &mut bundles,
            )?,
            MeasurementMode::Process => Self::measure_process(
                scenario,
                &test,
                iterations,
                sleep,
                &affinity,
                niceness,
                output_dir,
                &mut bundles,
            )?,
        };
        info!("  Test completed");

        Ok(())
    }

    fn reset_and_enable_bundles(
        bundles: &mut Vec<Box<dyn Bundle>>,
        stage: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            bundle
                .reset()
                .map_err(|e| format!("Failed to reset counters during {}: {}", stage, e))?;
            bundle
                .enable()
                .map_err(|e| format!("Failed to enable counters during {}: {}", stage, e))?;
        }
        Ok(())
    }

    fn disable_bundles(
        bundles: &mut Vec<Box<dyn Bundle>>,
        stage: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            bundle
                .disable()
                .map_err(|e| format!("Failed to disable counters during {}: {}", stage, e))?;
        }
        Ok(())
    }

    fn populate_measurement(
        measurement: &mut Measurement,
        bundles: &mut Vec<Box<dyn Bundle>>,
        stage: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            let data = bundle
                .read()
                .map_err(|e| format!("Failed to read bundle during {}: {}", stage, e))?;

            for (key, value) in data {
                match key.as_str() {
                    "time" => measurement.time = value.parse().ok(),
                    "energy-pkg" => measurement.pkg = value.parse().ok(),
                    "energy-cores" => measurement.cores = value.parse().ok(),
                    "energy-gpu" => measurement.gpu = value.parse().ok(),
                    "energy-ram" => measurement.ram = value.parse().ok(),
                    "energy-psys" => measurement.psys = value.parse().ok(),
                    "cycles" => measurement.cycles = value.parse().ok(),
                    "l1d_misses" => measurement.l1d_misses = value.parse().ok(),
                    "l1i_misses" => measurement.l1i_misses = value.parse().ok(),
                    "llc_misses" => measurement.llc_misses = value.parse().ok(),
                    "branch_misses" => measurement.branch_misses = value.parse().ok(),
                    "cstate_core/c1-residency" => {
                        measurement.c1_core_residency = value.parse().ok()
                    }
                    "cstate_core/c3-residency" => {
                        measurement.c3_core_residency = value.parse().ok()
                    }
                    "cstate_core/c6-residency" => {
                        measurement.c6_core_residency = value.parse().ok()
                    }
                    "cstate_core/c7-residency" => {
                        measurement.c7_core_residency = value.parse().ok()
                    }
                    "cstate_pkg/c2-residency" => measurement.c2_pkg_residency = value.parse().ok(),
                    "cstate_pkg/c3-residency" => measurement.c3_pkg_residency = value.parse().ok(),
                    "cstate_pkg/c6-residency" => measurement.c6_pkg_residency = value.parse().ok(),
                    "cstate_pkg/c8-residency" => measurement.c8_pkg_residency = value.parse().ok(),
                    "cstate_pkg/c10-residency" => {
                        measurement.c10_pkg_residency = value.parse().ok()
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn validate_output(output: &Output) -> Result<(), Box<dyn std::error::Error>> {
        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                error!("    Execution stderr:\n{}", stderr.trim());
            }
            return Err(format!("Execution failed with exit code {}", exit_code).into());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            warn!("    Execution warnings:\n{}", stderr.trim());
        }
        Ok(())
    }

    fn measure_internal(
        scenario: &mut Scenario,
        test: &Test,
        iterations: usize,
        sleep: u8,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        output_dir: &PathBuf,
        bundles: &mut Vec<Box<dyn Bundle>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        signal::set_iterations(iterations)?;

        let mut command = scenario.exec_command(test, output_dir, affinity, niceness)?;
        let child = command.spawn()?;

        for i in 1..=iterations {
            if sleep > 0 {
                info!("    Sleeping {} seconds", sleep);
                std::thread::sleep(std::time::Duration::from_secs(sleep as u64));
            }
            info!("    Iteration {}/{} started", i, iterations);

            signal::wait_for_start();

            Self::reset_and_enable_bundles(bundles, "iteration")?;

            signal::wait_for_end();

            Self::disable_bundles(bundles, "iteration")?;

            let mut measurement = Measurement::new(
                scenario,
                test,
                MeasurementMode::Internal,
                i,
                affinity,
                niceness,
            );

            Self::populate_measurement(&mut measurement, bundles, "iteration")?;

            measurement.write_to_csv(output_dir)?;
            info!("    Iteration {}/{} completed", i, iterations);
        }

        signal::cleanup_pipes();

        let output = child.wait_with_output()?;

        Self::validate_output(&output)?;
        Self::verify_iteration(scenario, test, iterations, output_dir)?;

        Ok(())
    }

    fn measure_external(
        scenario: &mut Scenario,
        test: &Test,
        iterations: usize,
        sleep: u8,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        output_dir: &PathBuf,
        bundles: &mut Vec<Box<dyn Bundle>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for i in 1..=iterations {
            let mut command = scenario.exec_command(test, output_dir, affinity, niceness)?;

            signal::set_iterations(1)?;

            if sleep > 0 {
                info!("    Sleeping {} seconds", sleep);
                std::thread::sleep(std::time::Duration::from_secs(sleep as u64));
            }
            info!("    Iteration {}/{} started", i, iterations);

            let child = command.spawn()?;

            signal::wait_for_start();

            Self::reset_and_enable_bundles(bundles, "iteration")?;

            signal::wait_for_end();

            Self::disable_bundles(bundles, "iteration")?;

            signal::cleanup_pipes();

            let output = child.wait_with_output()?;

            Self::validate_output(&output)?;

            let mut measurement = Measurement::new(
                scenario,
                test,
                MeasurementMode::External,
                i,
                affinity,
                niceness,
            );

            Self::populate_measurement(&mut measurement, bundles, "iteration")?;

            measurement.write_to_csv(output_dir)?;

            Self::verify_iteration(scenario, test, 1, output_dir)?;
            info!("    Iteration {}/{} completed", i, iterations);
        }

        Ok(())
    }

    fn measure_process(
        scenario: &mut Scenario,
        test: &Test,
        iterations: usize,
        sleep: u8,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        output_dir: &PathBuf,
        bundles: &mut Vec<Box<dyn Bundle>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for i in 1..=iterations {
            let mut command = scenario.exec_command(test, output_dir, affinity, niceness)?;

            if sleep > 0 {
                info!("    Sleeping {} seconds", sleep);
                std::thread::sleep(std::time::Duration::from_secs(sleep as u64));
            }
            info!("    Iteration {}/{} started", i, iterations);

            Self::reset_and_enable_bundles(bundles, "iteration")?;

            let child = command.spawn()?;
            let output = child.wait_with_output()?;

            Self::disable_bundles(bundles, "iteration")?;

            Self::validate_output(&output)?;

            let mut measurement = Measurement::new(
                scenario,
                test,
                MeasurementMode::Process,
                i,
                affinity,
                niceness,
            );

            Self::populate_measurement(&mut measurement, bundles, "iteration")?;

            measurement.write_to_csv(output_dir)?;

            Self::verify_iteration(scenario, test, 1, output_dir)?;
            info!("    Iteration {}/{} completed", i, iterations);
        }

        Ok(())
    }
}

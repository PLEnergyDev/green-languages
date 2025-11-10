use crate::core::util::results_dir;
use crate::core::{MeasurementMode, Scenario, ScenarioResult, Test};
use crate::{MeasureCommand, Measurement};
use csv::WriterBuilder;
use log::{error, info, warn};
use measurements::signal;
use nix::sched::{sched_setaffinity, CpuSet};
use nix::unistd::Pid;
use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Dynamic, Hardware, Software};
use perf_event::{Builder, Counter, Group};
use perf_event_data::ReadFormat;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Child, Output};
use std::time::Instant;

trait Bundle {
    fn new(pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>>
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
        pid: Option<i32>,
    ) -> Result<Vec<Box<dyn Bundle>>, Box<dyn std::error::Error>> {
        let mut bundles: Vec<Box<dyn Bundle>> = vec![];

        bundles.push(Box::new(TimeBundle::new(None)?));

        if self.rapl {
            bundles.push(Box::new(RaplBundle::new(None)?));
        }
        if self.misses {
            bundles.push(Box::new(MissesBundle::new(pid)?));
        }
        if self.cstates {
            bundles.push(Box::new(CStateBundle::new(None)?));
        }
        if self.cycles {
            bundles.push(Box::new(CyclesBundle::new(pid)?));
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
    counters: HashMap<&'static str, Counter>,
}

struct CStateBundle {
    counters: HashMap<String, Counter>,
}

struct CyclesBundle {
    counter: Counter,
}

struct TimeBundle {
    start_time: Option<Instant>,
}

impl Bundle for TimeBundle {
    fn new(_pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>> {
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
        let mut results = HashMap::new();
        if let Some(start) = self.start_time {
            let elapsed_micros = start.elapsed().as_micros() as u64;
            results.insert("time".to_string(), elapsed_micros.to_string());
        }
        Ok(results)
    }
}

impl Bundle for RaplBundle {
    fn new(_pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>> {
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
        let mut results = HashMap::new();
        for (name, rapl_counter) in &mut self.counters {
            let raw = rapl_counter.counter.read()?;
            let scaled = raw as f64 * rapl_counter.scale;
            results.insert(name.to_string(), format!("{:.3}", scaled));
        }
        Ok(results)
    }
}

impl Bundle for MissesBundle {
    fn new(pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut counters = HashMap::new();

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

        if let Some(process_id) = pid {
            if let Ok(l1d_counter) = Builder::new(L1D_MISS)
                .observe_pid(process_id)
                .inherit(true)
                .exclude_kernel(false)
                .exclude_hv(false)
                .enable_on_exec(true)
                .build()
            {
                counters.insert("l1d_misses", l1d_counter);
            }
            if let Ok(l1i_counter) = Builder::new(L1I_MISS)
                .observe_pid(process_id)
                .inherit(true)
                .exclude_kernel(false)
                .exclude_hv(false)
                .enable_on_exec(true)
                .build()
            {
                counters.insert("l1i_misses", l1i_counter);
            }
            if let Ok(llc_counter) = Builder::new(LLC_MISS)
                .observe_pid(process_id)
                .inherit(true)
                .exclude_kernel(false)
                .exclude_hv(false)
                .enable_on_exec(true)
                .build()
            {
                counters.insert("llc_misses", llc_counter);
            }
            if let Ok(branch_counter) = Builder::new(Hardware::BRANCH_MISSES)
                .observe_pid(process_id)
                .inherit(true)
                .exclude_kernel(false)
                .exclude_hv(false)
                .enable_on_exec(true)
                .build()
            {
                counters.insert("branch_misses", branch_counter);
            }
        } else {
            if let Ok(l1d_counter) = Builder::new(L1D_MISS).build() {
                counters.insert("l1d_misses", l1d_counter);
            }
            if let Ok(l1i_counter) = Builder::new(L1I_MISS).build() {
                counters.insert("l1i_misses", l1i_counter);
            }
            if let Ok(llc_counter) = Builder::new(LLC_MISS).build() {
                counters.insert("llc_misses", llc_counter);
            }
            if let Ok(branch_counter) = Builder::new(Hardware::BRANCH_MISSES).build() {
                counters.insert("branch_misses", branch_counter);
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
        let mut results = HashMap::new();
        for (name, counter) in &mut self.counters {
            let value = counter.read()?;
            results.insert(name.to_string(), value.to_string());
        }
        Ok(results)
    }
}

impl Bundle for CStateBundle {
    fn new(_pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>> {
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
        let num_cpus = num_cpus::get_physical();

        for event_name in core_events {
            for cpu in 0..num_cpus {
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

        let mut results = HashMap::new();
        for (name, value) in aggregated {
            results.insert(name, value.to_string());
        }
        Ok(results)
    }
}

impl Bundle for CyclesBundle {
    fn new(pid: Option<i32>) -> Result<Self, Box<dyn std::error::Error>> {
        let counter = if let Some(process_id) = pid {
            Builder::new(Hardware::CPU_CYCLES)
                .exclude_kernel(false)
                .exclude_hv(false)
                .observe_pid(process_id)
                .inherit(true)
                .enable_on_exec(true)
                .build()?
        } else {
            Builder::new(Hardware::CPU_CYCLES)
                .exclude_kernel(false)
                .exclude_hv(false)
                .build()?
        };

        Ok(Self { counter })
    }

    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.counter.enable()?;
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.counter.disable()?;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.counter.reset()?;
        Ok(())
    }

    fn read(&mut self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut results = HashMap::new();
        let cycles = self.counter.read()?;
        results.insert("cycles".to_string(), cycles.to_string());
        Ok(results)
    }
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

    fn new(scenario: &Scenario, test: &Test, mode: MeasurementMode, iteration: usize) -> Self {
        Self {
            language: scenario.language.to_string(),
            scenario: scenario.name.clone(),
            test: test.name.as_ref().unwrap().clone(),
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
    pub fn handle() -> Result<(), Box<dyn std::error::Error>> {
        let args = <Self as clap::Parser>::parse();

        let bundle_config = BundleConfig {
            rapl: args.rapl,
            misses: args.misses,
            cstates: args.cstates,
            cycles: args.cycles,
        };

        fn write_measurements(
            measurements: Vec<Measurement>,
            output_path: &PathBuf,
        ) -> Result<(), Box<dyn std::error::Error>> {
            for measurement in measurements {
                measurement.write_to_csv(output_path)?;
            }
            Ok(())
        }

        for scenario_path_str in &args.scenarios {
            let scenario_path = scenario_path_str.as_path();
            let mut scenario = Scenario::try_from(scenario_path)?;
            let tests = Test::iterate_from_file(scenario_path)?.peekable();
            let iterations: usize = args.iterations.into();
            let scenario_dir = scenario.scenario_dir();
            let mut has_tests = false;

            if scenario_dir.exists() {
                fs::remove_dir_all(&scenario_dir)?;
            }

            let output_path = if let Some(ref user_path) = args.output {
                if let Some(parent) = user_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                user_path.clone()
            } else {
                results_dir().join("results.csv")
            };

            for (index, test_result) in tests.enumerate() {
                has_tests = true;
                let mut test = test_result?;

                if test.name.is_none() {
                    test.name = Some((index + 1).to_string());
                }

                if let Ok(measurements) =
                    Self::process_test(&mut scenario, test, &bundle_config, 0, iterations)
                {
                    write_measurements(measurements, &output_path)?;

                    if args.sleep > 0 {
                        info!("Sleeping for {}", args.sleep);
                        std::thread::sleep(std::time::Duration::from_secs(args.sleep as u64));
                    }
                }
            }

            if !has_tests {
                let mut test = Test::default();
                test.name = Some("1".to_string());

                if let Ok(measurements) =
                    Self::process_test(&mut scenario, test, &bundle_config, 0, iterations)
                {
                    write_measurements(measurements, &output_path)?;

                    if args.sleep > 0 {
                        info!("Sleeping for {}", args.sleep);
                        std::thread::sleep(std::time::Duration::from_secs(args.sleep as u64));
                    }
                }
            }
        }

        Ok(())
    }

    fn process_test(
        scenario: &mut Scenario,
        mut test: Test,
        bundle_config: &BundleConfig,
        iter_index: usize,
        iterations: usize,
    ) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
        let test_name = test.name.as_ref().unwrap();
        let measurement_mode = test
            .measurement_mode
            .or(scenario.measurement_mode)
            .unwrap_or(MeasurementMode::Process);
        let context = format!(
            "[{}/{}/{}/{}]",
            scenario.language, scenario.name, test_name, measurement_mode
        );
        let affinity = test.affinity.clone().or(scenario.affinity.clone());
        let niceness = test.niceness.or(scenario.niceness);

        info!("{} Build started", context);
        match scenario.build_test(&mut test, iter_index) {
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
                return Err(format!("Build failed with exit code {}", exit_code).into());
            }
            Err(err) => {
                error!("{} Build error: {}", context, err);
                return Err(format!("Build error: {}", err).into());
            }
        }

        info!("{} Measurement start", context);
        let measurements = match measurement_mode {
            MeasurementMode::Internal => Self::measure_internal(
                scenario,
                &test,
                bundle_config,
                iterations,
                &affinity,
                niceness,
                &context,
            )?,
            MeasurementMode::External => Self::measure_external(
                scenario,
                &test,
                bundle_config,
                iterations,
                &affinity,
                niceness,
                &context,
            )?,
            MeasurementMode::Process => Self::measure_process(
                scenario,
                &test,
                bundle_config,
                iterations,
                &affinity,
                niceness,
                &context,
            )?,
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
                    return Err(format!("Test failed with exit code {}", exit_code).into());
                }
                Err(err) => {
                    error!("{} Test error: {}", context, err);
                    return Err(format!("Test error: {}", err).into());
                }
            }
        }

        Ok(measurements)
    }

    fn reset_and_enable_bundles(
        bundles: &mut Vec<Box<dyn Bundle>>,
        context: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            bundle
                .reset()
                .map_err(|e| format!("{} Failed to reset event counters: {}", context, e))?;
            bundle
                .enable()
                .map_err(|e| format!("{} Failed to enable event counters: {}", context, e))?;
        }
        Ok(())
    }

    fn disable_bundles(
        bundles: &mut Vec<Box<dyn Bundle>>,
        context: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            bundle
                .disable()
                .map_err(|e| format!("{} Failed to disable event counters: {}", context, e))?;
        }
        Ok(())
    }

    fn populate_measurement(
        measurement: &mut Measurement,
        bundles: &mut Vec<Box<dyn Bundle>>,
        context: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for bundle in bundles.iter_mut() {
            let data = bundle
                .read()
                .map_err(|e| format!("{} Failed to read bundle: {}", context, e))?;

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

    fn validate_output(output: &Output, context: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("{} Exec failed with exit code {}", context, exit_code);
            if !stderr.trim().is_empty() {
                error!("{} Exec stderr:\n{}", context, stderr.trim());
            }
            return Err(format!("Exec failed with exit code {}", exit_code).into());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            warn!("{} Exec stderr (warnings):\n{}", context, stderr.trim());
        }
        Ok(())
    }

    fn measure_internal(
        scenario: &mut Scenario,
        test: &Test,
        bundle_config: &BundleConfig,
        iterations: usize,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        context: &str,
    ) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
        let mut measurements = vec![];

        signal::set_iterations(iterations)?;

        let child = scenario.exec_test_async(test)?;
        let pid = child.id() as i32;

        let mut bundles = bundle_config.create_bundles(Some(pid))?;

        Self::configure_process(&child, affinity, niceness)?;

        for i in 1..=iterations {
            signal::wait_for_start();

            Self::reset_and_enable_bundles(&mut bundles, context)?;

            signal::wait_for_end();

            Self::disable_bundles(&mut bundles, context)?;

            let mut measurement = Measurement::new(scenario, test, MeasurementMode::Internal, i);

            Self::populate_measurement(&mut measurement, &mut bundles, context)?;

            measurements.push(measurement);
        }

        signal::cleanup_pipes();

        let output = child.wait_with_output()?;

        Self::validate_output(&output, context)?;

        Ok(measurements)
    }

    fn measure_external(
        scenario: &mut Scenario,
        test: &Test,
        bundle_config: &BundleConfig,
        iterations: usize,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        context: &str,
    ) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
        let mut measurements = Vec::new();

        for i in 1..=iterations {
            signal::set_iterations(1)?;

            let child = scenario.exec_test_async(test)?;
            let pid = child.id() as i32;

            let mut iter_bundles = bundle_config.create_bundles(Some(pid))?;

            Self::configure_process(&child, affinity, niceness)?;

            signal::wait_for_start();

            Self::reset_and_enable_bundles(&mut iter_bundles, context)?;

            signal::wait_for_end();

            Self::disable_bundles(&mut iter_bundles, context)?;

            signal::cleanup_pipes();

            let output = child.wait_with_output()?;

            Self::validate_output(&output, context)?;

            let mut measurement = Measurement::new(scenario, test, MeasurementMode::External, i);

            Self::populate_measurement(&mut measurement, &mut iter_bundles, context)?;

            measurements.push(measurement);
        }

        Ok(measurements)
    }

    fn measure_process(
        scenario: &mut Scenario,
        test: &Test,
        bundle_config: &BundleConfig,
        iterations: usize,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
        context: &str,
    ) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
        let mut measurements = Vec::new();

        for i in 1..=iterations {
            let child = scenario.exec_test_async(test)?;
            let pid = child.id() as i32;

            let mut iter_bundles = bundle_config.create_bundles(Some(pid))?;

            Self::configure_process(&child, affinity, niceness)?;

            Self::reset_and_enable_bundles(&mut iter_bundles, context)?;

            let output = child.wait_with_output()?;

            Self::disable_bundles(&mut iter_bundles, context)?;

            Self::validate_output(&output, context)?;

            let mut measurement = Measurement::new(scenario, test, MeasurementMode::Process, i);

            Self::populate_measurement(&mut measurement, &mut iter_bundles, context)?;

            measurements.push(measurement);
        }

        Ok(measurements)
    }

    fn configure_process(
        child: &Child,
        affinity: &Option<Vec<usize>>,
        niceness: Option<i32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(cpus) = affinity {
            let pid = Pid::from_raw(child.id() as i32);
            let mut cpu_set = CpuSet::new();

            for &cpu in cpus {
                cpu_set
                    .set(cpu)
                    .map_err(|e| format!("Failed to add CPU {} to affinity set: {}", cpu, e))?;
            }
            sched_setaffinity(pid, &cpu_set)
                .map_err(|e| format!("Failed to set CPU affinity: {}", e))?;
        }

        if let Some(nice_value) = niceness {
            unsafe {
                if libc::setpriority(libc::PRIO_PROCESS, child.id(), nice_value) != 0 {
                    return Err("Failed to set process priority".into());
                }
            }
        }

        Ok(())
    }
}

use crate::core::scenario::PreparedCommand;
use crate::core::util::Measurement;
use crate::core::{MeasurementMode, Scenario, ScenarioResult, Test};
use crate::{MeasureCommand, RawMeasurement};
use csv::WriterBuilder;
use log::{error, info, warn};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Output;

impl MeasureCommand {
    pub fn metrics_string(&self) -> String {
        let mut metrics = vec![];
        if self.rapl {
            metrics.push("rapl");
        }
        if self.cycles {
            metrics.push("cycles");
        }
        if self.misses {
            metrics.push("misses");
        }
        if self.cstates {
            metrics.push("cstates");
        }
        metrics.join(",")
    }

    pub fn handle(args: Self) -> Result<(), Box<dyn std::error::Error>> {
        let output_dir = args.output.clone().unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&output_dir)?;
        let metrics = args.metrics_string();

        for scenario_path_str in &args.scenarios {
            let scenario_path = scenario_path_str.as_path();
            let mut scenario = Scenario::try_from(scenario_path)?;

            let scenario_dir = scenario.scenario_dir(&output_dir);
            if scenario_dir.exists() {
                fs::remove_dir_all(&scenario_dir)?;
            }

            let mut tests = Test::iterate_from_file(scenario_path)?.peekable();

            if tests.peek().is_none() {
                if let Err(err) = Self::process_test(
                    &mut scenario,
                    Test::default(),
                    args.runs,
                    args.internal_runs,
                    args.cooldown,
                    &metrics,
                    &output_dir,
                ) {
                    error!("{}", err);
                }
            } else {
                for (test_index, test_result) in tests.enumerate() {
                    let mut test = test_result?;
                    if test.name.is_none() {
                        test.name = Some((test_index + 1).to_string());
                    }
                    if let Err(err) = Self::process_test(
                        &mut scenario,
                        test,
                        args.runs,
                        args.internal_runs,
                        args.cooldown,
                        &metrics,
                        &output_dir,
                    ) {
                        error!("{}", err);
                    }
                }
            }
        }

        Ok(())
    }

    fn verify_output(
        scenario: &mut Scenario,
        test: &Test,
        internal_runs: usize,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match scenario.verify_test(test, internal_runs, output_dir) {
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
        runs: usize,
        internal_runs: usize,
        cooldown: u64,
        metrics: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let test_name = test.name.as_ref().unwrap();

        let mode = if scenario.libgreen.unwrap_or(false) {
            MeasurementMode::Internal
        } else {
            MeasurementMode::Process
        };

        let affinity = test.affinity.clone().or(scenario.affinity.clone());
        let nice = test.nice.or(scenario.nice);

        let affinity_str = affinity
            .as_ref()
            .map(|a| {
                a.iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "-".to_string());
        let nice_str = nice
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());

        info!(
            "[{} | {} | {} | {} | {}@{}]",
            scenario.language, scenario.name, test_name, mode, nice_str, affinity_str
        );
        info!("  Build started");

        match scenario.build_test(&mut test, output_dir) {
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

        info!(
            "  Test started ({} runs, {} internal runs)",
            runs, internal_runs
        );
        Self::measure(
            scenario,
            &test,
            runs,
            internal_runs,
            cooldown,
            metrics,
            output_dir,
        )?;
        info!("  Test completed");

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

    fn write_measurements(
        scenario: &Scenario,
        test: &Test,
        mode: MeasurementMode,
        run: usize,
        measurement_path: &PathBuf,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let affinity = test
            .affinity
            .as_ref()
            .or(scenario.affinity.as_ref())
            .map(|a| {
                a.iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            });
        let nice = test.nice.or(scenario.nice);

        let csv_path = output_dir.join("measurements.csv");
        let file_exists = csv_path.exists();
        let out_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&csv_path)?;
        let mut writer = WriterBuilder::new()
            .has_headers(!file_exists)
            .from_writer(out_file);

        let mut reader = csv::Reader::from_path(measurement_path)?;
        let mut internal_run = 1usize;
        for result in reader.deserialize::<RawMeasurement>() {
            let raw = result?;
            writer.serialize(crate::Measurement {
                scenario: scenario.name.clone(),
                language: scenario.language.to_string(),
                test: test.name.clone().unwrap_or_default(),
                nice: nice,
                affinity: affinity.clone(),
                mode,
                run,
                internal_run,
                time: raw.time,
                pkg: raw.pkg,
                cores: raw.cores,
                gpu: raw.gpu,
                ram: raw.ram,
                psys: raw.psys,
                cycles: raw.cycles,
                l1d_misses: raw.l1d_misses,
                l1i_misses: raw.l1i_misses,
                llc_misses: raw.llc_misses,
                branch_misses: raw.branch_misses,
                c1_core_residency: raw.c1_core_residency,
                c3_core_residency: raw.c3_core_residency,
                c6_core_residency: raw.c6_core_residency,
                c7_core_residency: raw.c7_core_residency,
                c2_pkg_residency: raw.c2_pkg_residency,
                c3_pkg_residency: raw.c3_pkg_residency,
                c6_pkg_residency: raw.c6_pkg_residency,
                c8_pkg_residency: raw.c8_pkg_residency,
                c10_pkg_residency: raw.c10_pkg_residency,
                ended: raw.ended,
            })?;
            internal_run += 1;
        }

        writer.flush()?;
        fs::remove_file(measurement_path)?;
        Ok(())
    }

    fn measure(
        scenario: &mut Scenario,
        test: &Test,
        runs: usize,
        internal_runs: usize,
        cooldown: u64,
        metrics: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for run in 1..=runs {
            if cooldown > 0 {
                info!("    Cooldown {}ms", cooldown);
                std::thread::sleep(std::time::Duration::from_millis(cooldown));
            }

            info!("    Run {}/{} started", run, runs);

            let PreparedCommand {
                mut command,
                metrics: metrics_str,
                measurement_path,
                mode,
            } = scenario.exec_command(test, internal_runs, metrics, output_dir)?;
            let child = command.spawn()?;

            let output = match mode {
                MeasurementMode::Process => {
                    unsafe {
                        std::env::set_var("LG_OUTPUT", &measurement_path);
                    }
                    let _context = Measurement::start(&metrics_str);
                    let result = child.wait_with_output()?;
                    unsafe {
                        std::env::remove_var("LG_OUTPUT");
                    }
                    result
                }
                MeasurementMode::Internal => child.wait_with_output()?,
            };

            if let Err(err) = Self::validate_output(&output) {
                let _ = fs::remove_file(&measurement_path);
                return Err(err);
            }

            if let Err(err) = Self::verify_output(scenario, test, internal_runs, output_dir) {
                let _ = fs::remove_file(&measurement_path);
                return Err(err);
            }

            if measurement_path.exists() {
                if let Err(e) = Self::write_measurements(
                    scenario,
                    test,
                    mode,
                    run,
                    &measurement_path,
                    output_dir,
                ) {
                    error!("    Failed to write measurements: {}", e);
                }
            }

            info!("    Run {}/{} completed", run, runs);
        }

        Ok(())
    }
}


use crate::MeasureArgs;
use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Dynamic, Hardware, Software};
use perf_event::{Builder, Counter, Group};
use perf_event_data::ReadFormat;
use std::collections::HashMap;

struct RaplCounter {
    counter: Counter,
    scale: f64,
}

struct RaplBundle {
    group: Group,
    counters: HashMap<&'static str, RaplCounter>,
}

struct MissesBundle {
    group: Group,
    counters: HashMap<&'static str, Counter>,
}

struct CStateBundle {
    counters: HashMap<String, Counter>,
}

impl RaplBundle {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let rapl_events = vec![
            "energy-pkg",
            "energy-cores",
            "energy-gpu",
            "energy-psys",
            "energy-dram",
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

    pub fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.enable()?;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.disable()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.reset()?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
        let mut results = HashMap::new();
        for (name, rapl_counter) in &mut self.counters {
            let raw = rapl_counter.counter.read()?;
            let scaled = raw as f64 * rapl_counter.scale;
            results.insert(name.to_string(), scaled);
        }
        Ok(results)
    }
}

impl MissesBundle {
    pub fn new(
        cache_misses: bool,
        branch_misses: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut group = Group::new()?;
        let mut counters = HashMap::new();

        if cache_misses {
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

            if let Ok(l1d_counter) = group.add(&Builder::new(L1D_MISS)) {
                counters.insert("l1d_misses", l1d_counter);
            }
            if let Ok(l1i_counter) = group.add(&Builder::new(L1I_MISS)) {
                counters.insert("l1i_misses", l1i_counter);
            }
            if let Ok(llc_counter) = group.add(&Builder::new(LLC_MISS)) {
                counters.insert("llc_misses", llc_counter);
            }
        }
        if branch_misses {
            if let Ok(branch_counter) = group.add(&Builder::new(Hardware::BRANCH_MISSES)) {
                counters.insert("branch_misses", branch_counter);
            }
        }

        Ok(Self { group, counters })
    }

    pub fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.enable()?;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.disable()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.group.reset()?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
        let mut results = HashMap::new();
        for (name, counter) in &mut self.counters {
            let value = counter.read()?;
            results.insert(name.to_string(), value);
        }
        Ok(results)
    }
}

impl CStateBundle {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
        let num_cpus = num_cpus::get();

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

    pub fn enable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.enable()?;
        }
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.disable()?;
        }
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for counter in self.counters.values_mut() {
            counter.reset()?;
        }
        Ok(())
    }

    pub fn read(&mut self) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
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
                s if s.starts_with("cstate_pkg/") => {
                    let pkg_event = s
                        .strip_prefix("cstate_pkg/")
                        .and_then(|s| s.split('_').next())
                        .unwrap_or(s);
                    format!("cstate_pkg/{}", pkg_event)
                }
                _ => name.clone(),
            };

            *aggregated.entry(event_name).or_insert(0) += value;
        }

        Ok(aggregated)
    }
}

impl MeasureArgs {
    pub fn handle_args() -> Result<(), Box<dyn std::error::Error>> {
        let args = <Self as clap::Parser>::parse();
        let mut rapl = if args.rapl {
            Some(RaplBundle::new()?)
        } else {
            None
        };

        let mut hardware = if args.cache_misses || args.branch_misses {
            Some(MissesBundle::new(args.cache_misses, args.branch_misses)?)
        } else {
            None
        };

        let mut cstates = if args.cstates {
            Some(CStateBundle::new()?)
        } else {
            None
        };

        let duration = std::env::args()
            .nth(1)
            .and_then(|arg| arg.parse().ok())
            .unwrap_or(1.0);

        if let Some(ref mut r) = rapl {
            r.enable()?;
        }
        if let Some(ref mut h) = hardware {
            h.enable()?;
        }
        if let Some(ref mut c) = cstates {
            c.enable()?;
        }

        std::thread::sleep(std::time::Duration::from_secs_f64(duration));

        if let Some(ref mut r) = rapl {
            r.disable()?;
        }
        if let Some(ref mut h) = hardware {
            h.disable()?;
        }
        if let Some(ref mut c) = cstates {
            c.disable()?;
        }

        if let Some(ref mut r) = rapl {
            println!("RAPL Energy Measurements:");
            for (name, value) in r.read()? {
                println!("  {}: {:.3} J", name, value);
            }
        }

        if let Some(ref mut h) = hardware {
            println!("\nHardware Counters:");
            for (name, value) in h.read()? {
                println!("  {}: {}", name, value);
            }
        }

        if let Some(ref mut c) = cstates {
            println!("\nCstate Counters:");
            for (name, value) in c.read()? {
                println!("  {}: {}", name, value);
            }
        }

        Ok(())
    }
}

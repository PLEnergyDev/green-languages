use crate::share::SharedMemory;
use once_cell::sync::Lazy;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SHARED_MEM: Lazy<Mutex<Option<SharedMemory>>> = Lazy::new(|| Mutex::new(None));

pub fn init_shared_state() -> Result<(), Box<dyn std::error::Error>> {
    let shm = SharedMemory::create()?;
    *SHARED_MEM.lock().unwrap() = Some(shm);
    Ok(())
}

pub fn set_iterations(count: usize) {
    if let Some(shm) = SHARED_MEM.lock().unwrap().as_ref() {
        let state = shm.get();
        state.iterations.store(count, Ordering::SeqCst);
        state.should_start.store(false, Ordering::SeqCst);
        state.measuring.store(false, Ordering::SeqCst);
        state.ready.store(false, Ordering::SeqCst);
    }
}

pub fn get_iterations() -> usize {
    SHARED_MEM
        .lock()
        .unwrap()
        .as_ref()
        .map(|shm| shm.get().iterations.load(Ordering::SeqCst))
        .unwrap_or(0)
}

fn wait_for_flag(flag_check: impl Fn(&crate::share::SharedState) -> bool) {
    if let Some(shm) = SHARED_MEM.lock().unwrap().as_ref() {
        let state = shm.get();
        while !flag_check(state) {
            std::thread::sleep(Duration::from_micros(100));
        }
    }
}

pub fn wait_for_ready() {
    wait_for_flag(|state| state.ready.load(Ordering::SeqCst));
}

pub fn wait_for_measuring() {
    wait_for_flag(|state| state.measuring.load(Ordering::SeqCst));
}

pub fn wait_for_complete() {
    wait_for_flag(|state| !state.measuring.load(Ordering::SeqCst));
}

pub fn signal_proceed() {
    if let Some(shm) = SHARED_MEM.lock().unwrap().as_ref() {
        let state = shm.get();
        state.should_start.store(true, Ordering::SeqCst);
        state.ready.store(false, Ordering::SeqCst);
    }
}

pub fn next_iteration() -> i32 {
    let shm = match SharedMemory::open() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let state = shm.get();

    loop {
        let current = state.iterations.load(Ordering::SeqCst);
        if current == 0 {
            return 0;
        }

        if state
            .iterations
            .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            break;
        }
    }

    state.ready.store(true, Ordering::SeqCst);

    let start = Instant::now();
    while !state.should_start.load(Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(60) {
            return 0;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    state.measuring.store(true, Ordering::SeqCst);
    state.should_start.store(false, Ordering::SeqCst);
    1
}

pub fn mark_end() {
    if let Ok(shm) = SharedMemory::open() {
        shm.get().measuring.store(false, Ordering::SeqCst);
    }
}

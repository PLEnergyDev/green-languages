use std::os::unix::io::RawFd;
use std::sync::Mutex;

static PARENT_STATE: Mutex<Option<ParentState>> = Mutex::new(None);

struct ParentState {
    control_write_fd: RawFd,
    status_read_fd: RawFd,
}

static CHILD_STATE: Mutex<Option<ChildState>> = Mutex::new(None);

struct ChildState {
    control_fd: RawFd,
    status_fd: RawFd,
}

pub fn set_iterations(iterations: usize) -> std::io::Result<()> {
    let mut control_pipe = [-1i32; 2];
    let mut status_pipe = [-1i32; 2];

    unsafe {
        if libc::pipe(control_pipe.as_mut_ptr()) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::pipe(status_pipe.as_mut_ptr()) != 0 {
            libc::close(control_pipe[0]);
            libc::close(control_pipe[1]);
            return Err(std::io::Error::last_os_error());
        }
    }

    let control_read_fd = control_pipe[0];
    let control_write_fd = control_pipe[1];
    let status_read_fd = status_pipe[0];
    let status_write_fd = status_pipe[1];

    std::env::set_var("MEASUREMENT_CONTROL_FD", control_read_fd.to_string());
    std::env::set_var("MEASUREMENT_STATUS_FD", status_write_fd.to_string());
    std::env::set_var("MEASUREMENT_ITERATIONS", iterations.to_string());

    *PARENT_STATE.lock().unwrap() = Some(ParentState {
        control_write_fd,
        status_read_fd,
    });

    Ok(())
}

pub fn wait_for_start() {
    let state_guard = PARENT_STATE.lock().unwrap();
    let state = state_guard
        .as_ref()
        .expect("Pipes not initialized - call set_iterations() first");

    let mut ready_signal = [0u8];
    unsafe {
        libc::read(
            state.status_read_fd,
            ready_signal.as_mut_ptr() as *mut libc::c_void,
            1,
        );
    }
}

pub fn wait_for_end() {
    let state_guard = PARENT_STATE.lock().unwrap();
    let state = state_guard.as_ref().expect("Pipes not initialized");

    let proceed_signal = [1u8];
    unsafe {
        libc::write(
            state.control_write_fd,
            proceed_signal.as_ptr() as *const libc::c_void,
            1,
        );
    }

    let mut done_signal = [0u8];
    unsafe {
        libc::read(
            state.status_read_fd,
            done_signal.as_mut_ptr() as *mut libc::c_void,
            1,
        );
    }
}

pub fn cleanup_pipes() {
    let mut state_guard = PARENT_STATE.lock().unwrap();
    if let Some(state) = state_guard.take() {
        let stop_signal = [0u8];
        unsafe {
            libc::write(
                state.control_write_fd,
                stop_signal.as_ptr() as *const libc::c_void,
                1,
            );
            libc::close(state.control_write_fd);
            libc::close(state.status_read_fd);
        }
    }

    std::env::remove_var("MEASUREMENT_CONTROL_FD");
    std::env::remove_var("MEASUREMENT_STATUS_FD");
    std::env::remove_var("MEASUREMENT_ITERATIONS");
}

fn initialize_child() -> Option<ChildState> {
    let control_fd = std::env::var("MEASUREMENT_CONTROL_FD")
        .ok()?
        .parse::<RawFd>()
        .ok()?;

    let status_fd = std::env::var("MEASUREMENT_STATUS_FD")
        .ok()?
        .parse::<RawFd>()
        .ok()?;

    Some(ChildState {
        control_fd,
        status_fd,
    })
}

pub fn start_measurement() -> i32 {
    let mut state_guard = CHILD_STATE.lock().unwrap();

    if state_guard.is_none() {
        *state_guard = initialize_child();
    }

    let state = match state_guard.as_mut() {
        Some(s) => s,
        None => return 0,
    };

    let ready_signal = [1u8];
    unsafe {
        let result = libc::write(
            state.status_fd,
            ready_signal.as_ptr() as *const libc::c_void,
            1,
        );
        if result != 1 {
            return 0;
        }
    }

    let mut proceed_signal = [0u8];
    unsafe {
        let result = libc::read(
            state.control_fd,
            proceed_signal.as_mut_ptr() as *mut libc::c_void,
            1,
        );
        if result != 1 {
            return 0;
        }
    }

    if proceed_signal[0] == 0 {
        return 0;
    }

    1
}

pub fn end_measurement() {
    let mut state_guard = CHILD_STATE.lock().unwrap();

    let state = match state_guard.as_mut() {
        Some(s) => s,
        None => return,
    };

    let done_signal = [1u8];
    unsafe {
        libc::write(
            state.status_fd,
            done_signal.as_ptr() as *const libc::c_void,
            1,
        );
    }
}

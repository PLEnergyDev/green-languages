use nix::fcntl::OFlag;
use nix::sys::mman::{shm_open, shm_unlink, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::{mem, ptr};

pub const SHM_NAME: &str = "/iterations-state";

#[repr(C)]
pub struct SharedState {
    pub measuring: AtomicBool,
    pub should_start: AtomicBool,
    pub iterations: AtomicUsize,
    pub ready: AtomicBool,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            measuring: AtomicBool::new(false),
            should_start: AtomicBool::new(false),
            iterations: AtomicUsize::new(0),
            ready: AtomicBool::new(false),
        }
    }
}

pub struct SharedMemory {
    ptr: NonNull<SharedState>,
    size: usize,
}

unsafe impl Send for SharedMemory {}
unsafe impl Sync for SharedMemory {}

impl SharedMemory {
    fn mmap_shared(
        fd: &impl std::os::fd::AsFd,
        size: usize,
    ) -> Result<NonNull<SharedState>, Box<dyn std::error::Error>> {
        let ptr = unsafe {
            nix::sys::mman::mmap(
                None,
                size.try_into()?,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0,
            )?
        };
        NonNull::new(ptr.as_ptr() as *mut SharedState).ok_or_else(|| "mmap returned null".into())
    }

    pub fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let _ = shm_unlink(SHM_NAME);

        let fd = shm_open(
            SHM_NAME,
            OFlag::O_CREAT | OFlag::O_RDWR,
            Mode::S_IRUSR | Mode::S_IWUSR,
        )?;

        let size = mem::size_of::<SharedState>();
        ftruncate(&fd, size as i64)?;

        let ptr = Self::mmap_shared(&fd, size)?;

        unsafe {
            ptr::write(ptr.as_ptr(), SharedState::default());
        }

        Ok(Self { ptr, size })
    }

    pub fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let fd = shm_open(SHM_NAME, OFlag::O_RDWR, Mode::empty())?;
        let size = mem::size_of::<SharedState>();
        let ptr = Self::mmap_shared(&fd, size)?;

        Ok(Self { ptr, size })
    }

    pub fn get(&self) -> &SharedState {
        unsafe { self.ptr.as_ref() }
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            let addr = self.ptr.cast::<std::ffi::c_void>();
            let _ = nix::sys::mman::munmap(addr, self.size);
        }
    }
}

pub fn cleanup_shared_memory() {
    let _ = shm_unlink(SHM_NAME);
}

//! Mutex for Apple platforms.
//!
//! On Apple platforms, priority inheritance is the default for locks. To avoid
//! having to use pthread's mutex, which needs some tricks to work correctly, we
//! instead use `os_unfair_lock`, which is small, movable and supports priority-
//! inheritance and appeared with macOS 10.12, which is exactly our minimum
//! supported version.

use crate::cell::SyncUnsafeCell;

// FIXME: move these definitions to libc

#[allow(non_camel_case_types)]
#[repr(C)]
struct os_unfair_lock {
    _opaque: u32,
}

const OS_UNFAIR_LOCK_INIT: os_unfair_lock = os_unfair_lock { _opaque: 0 };

extern "C" {
    fn os_unfair_lock_lock(lock: *mut os_unfair_lock);
    fn os_unfair_lock_trylock(lock: *mut os_unfair_lock) -> bool;
    fn os_unfair_lock_unlock(lock: *mut os_unfair_lock);
}

pub struct Mutex {
    lock: SyncUnsafeCell<os_unfair_lock>,
}

impl Mutex {
    pub const fn new() -> Mutex {
        Mutex { lock: SyncUnsafeCell::new(OS_UNFAIR_LOCK_INIT) }
    }

    pub fn lock(&self) {
        unsafe { os_unfair_lock_lock(self.lock.get()) }
    }

    pub fn try_lock(&self) -> bool {
        unsafe { os_unfair_lock_trylock(self.lock.get()) }
    }

    pub unsafe fn unlock(&self) {
        unsafe { os_unfair_lock_unlock(self.lock.get()) }
    }
}

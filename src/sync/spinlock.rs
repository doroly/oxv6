//! Interrupt-safe Spinlock implementation for multi-hart synchronization.

use crate::arch::riscv64::csr::{pop_off, push_off};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A kernel spinlock protecting inner data `T`.
pub(crate) struct SpinLock<T> {
    /// Atomic state flag (`true` if locked, `false` if unlocked).
    locked: AtomicBool,
    /// Diagnostic name for debugging deadlock panics.
    name: &'static str,
    /// Protected payload.
    data: UnsafeCell<T>,
}

// Guarantee thread safety across multiple harts for Send-able types.
unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock wrapping the provided data.
    pub(crate) const fn new(name: &'static str, data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            name,
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, spinning until available.
    /// Disables local interrupts while locked to prevent deadlocks.
    pub(crate) fn lock(&self) -> SpinLockGuard<'_, T> {
        // 1. Disable local interrupts on this hart before spinning
        push_off();

        // 2. Atomic swap spin-loop (Acquire ordering ensures memory ops don't reorder before lock)
        while self.locked.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }

        SpinLockGuard { lock: self }
    }

    /// Forces release of the lock. Called automatically by `SpinLockGuard` on drop.
    fn release(&self) {
        if !self.locked.load(Ordering::Relaxed) {
            panic!("spinlock '{}': release unlocked lock", self.name);
        }

        // Release ordering flushes writes before releasing lock
        self.locked.store(false, Ordering::Release);

        // Restore interrupt state on this hart
        pop_off();
    }
}

/// An RAII guard providing scoped mutable access to the locked data.
pub(crate) struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    /// Automatically release the lock when the guard goes out of scope.
    fn drop(&mut self) {
        self.lock.release();
    }
}

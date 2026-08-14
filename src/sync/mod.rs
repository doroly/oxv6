//! Kernel synchronization primitives.

pub(crate) mod spinlock;

pub(crate) use spinlock::SpinLock;
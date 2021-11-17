#![allow(dead_code)]
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

const LOCKED: bool = true;
const UNLOCKED: bool = false;

struct Mutex<T> {
    locked: AtomicBool,
    v: UnsafeCell<T>,
}

// we know that Mutex is Sync
unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            locked: AtomicBool::new(UNLOCKED),
            v: UnsafeCell::new(t),
        }
    }
    // We want to grab a lock and execute f
    pub fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.locked.load(Ordering::Relaxed) != UNLOCKED { /* spin lock*/ }
        // bug : maybe another thread runs here so it's possible for data race
        self.locked.store(LOCKED, Ordering::Relaxed);
        // Safety : we hold the lock so we can create mutable ref
        let ret = f(unsafe { &mut *self.v.get() });
        self.locked.store(UNLOCKED, Ordering::Relaxed);
        ret
    }
    // better implementation ( it still fails because of orderings )
    pub fn with_lock_2<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(
                // very inefficient but works ( all threads will fight to get that value )
                UNLOCKED,
                LOCKED,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_err()
        {
            // spin lock
            // MESI protocol
            // more efficient waiting if we fail with compare_exchange_weak
            while self.locked.load(Ordering::Relaxed) == LOCKED {}
        }
        // Safety : we hold the lock so we can create mutable ref
        let ret = f(unsafe { &mut *self.v.get() });
        self.locked.store(UNLOCKED, Ordering::Relaxed);
        ret
    }

    // Prevent reordering of operations with Orderings ( correct impl )
    pub fn with_lock_3<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(
                // very inefficient but works ( all threads will fight to get that value )
                UNLOCKED,
                LOCKED,
                Ordering::Acquire, // <- We acquire here
                Ordering::Relaxed, // <- We don't care in case of failure to acquire the lock
            )
            .is_err()
        {
            // spin lock
            // MESI protocol
            // more efficient waiting if we fail with compare_exchange
            while self.locked.load(Ordering::Relaxed) == LOCKED {}
        }
        // Safety : we hold the lock so we can create mutable ref
        let ret = f(unsafe { &mut *self.v.get() });
        self.locked.store(UNLOCKED, Ordering::Release); // <- Release here
        ret
    }
}

fn main() {
    println!("Hello, world!");
}

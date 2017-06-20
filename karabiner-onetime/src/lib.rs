#![feature(integer_atomics)]

extern crate parking_lot;

#[cfg(test)]
mod tests;

use std::ops::{Drop, Deref, DerefMut};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::{Mutex, MutexGuard};

pub struct State(u8);

pub const INIT: State = State(0);
pub const WAIT: State = State(1);
pub const FREE: State = State(2);

impl Deref for State {
    type Target = u8;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct Lock<T> {
    atom: AtomicU8,
    lock: Mutex<()>,
    cell: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for Lock<T> {}
unsafe impl<T: Sync> Sync for Lock<T> {}

impl<T> Lock<T>
where
    T: Send + Sync,
{
    pub fn new(inner: T) -> Lock<T> {
        Lock {
            atom: AtomicU8::new(*INIT),
            lock: Mutex::new(()),
            cell: UnsafeCell::new(inner),
        }
    }

    pub fn try_lock(&self) -> Option<LockGuard<T>> {
        // Ordering::Acquire
        if self.atom.compare_and_swap(*INIT, *WAIT, Ordering::SeqCst) == *INIT {
            Some(LockGuard::new(self))
        } else {
            None
        }
    }

    fn is_locked(&self) -> bool {
        // Ordering::Release
        self.atom.load(Ordering::SeqCst) == *WAIT
    }

    pub fn wait(&self) {
        if self.is_locked() {
            let _ = self.lock.lock();
        }
    }

    pub fn into_inner(self) -> T {
        // this is safe, because of `self`.
        unsafe { self.cell.into_inner() }
    }
}

impl<T> Deref for Lock<T>
where
    T: Send + Sync,
{
    type Target = T;

    /// Dereference to the value inside the Lock.
    /// This can block if the Lock is in its lock state.
    fn deref(&self) -> &T {
        if self.atom.compare_and_swap(*INIT, *FREE, Ordering::SeqCst) == *WAIT {
            self.wait();
        }
        debug_assert_eq!(self.atom.load(Ordering::SeqCst), *FREE);
        unsafe { &*self.cell.get() }
    }
}
impl<T> DerefMut for Lock<T>
where
    T: Send + Sync,
{
    fn deref_mut(&mut self) -> &mut T {
        // `&mut self` means no LockGuard's exist.
        debug_assert_ne!(self.atom.load(Ordering::SeqCst), *WAIT);
        unsafe { &mut *self.cell.get() }
    }
}

pub struct LockGuard<'a, T: 'a> {
    mutex: &'a Lock<T>,
    _guard: MutexGuard<'a, ()>,
}

impl<'a, T> LockGuard<'a, T>
where
    T: 'a,
{
    fn new(mutex: &'a Lock<T>) -> LockGuard<'a, T> {
        let _guard = mutex.lock.lock();
        LockGuard { mutex, _guard }
    }
}

impl<'a, T> ::std::fmt::Debug for LockGuard<'a, T>
where
    T: Send + Sync,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.pad("LockGuard")
    }
}
impl<'a, T> Deref for LockGuard<'a, T>
where
    T: Send + Sync,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.cell.get() }
    }
}
impl<'a, T> DerefMut for LockGuard<'a, T>
where
    T: Send + Sync,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.cell.get() }
    }
}
impl<'a, T> Drop for LockGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.atom.store(*FREE, Ordering::SeqCst);
    }
}

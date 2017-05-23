extern crate parking_lot;

#[cfg(test)]
mod tests;

use std::ops::{Drop, Deref, DerefMut};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::{Mutex, MutexGuard};

pub struct State(usize);

pub const INIT: State = State(0);
pub const WAIT: State = State(1);
pub const FREE: State = State(2);

impl Deref for State {
    type Target = usize;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct JustOnce<T> {
    atom: AtomicUsize,
    lock: Mutex<()>,
    cell: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for JustOnce<T> {}
unsafe impl<T: Sync> Sync for JustOnce<T> {}

impl<T> JustOnce<T>
    where T: Send + Sync
{
    pub fn new(inner: T) -> JustOnce<T> {
        JustOnce {
            atom: AtomicUsize::new(*INIT),
            lock: Mutex::new(()),
            cell: UnsafeCell::new(inner),
        }
    }

    pub fn try_lock(&self) -> Option<JustOnceGuard<T>> {
        // Ordering::Acquire
        if self.atom.compare_and_swap(*INIT, *WAIT, Ordering::SeqCst) == *INIT {
            Some(JustOnceGuard::new(self))
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
}

impl<T> Deref for JustOnce<T>
    where T: Send + Sync
{
    type Target = T;

    /// Dereference to the value inside the JustOnce.
    /// This can block if the JustOnce is in its lock state.
    fn deref(&self) -> &T {
        if self.atom.compare_and_swap(*INIT, *FREE, Ordering::SeqCst) == *WAIT {
            self.wait();
        }
        debug_assert_eq!(self.atom.load(Ordering::SeqCst), *FREE);
        unsafe { &*self.cell.get() }
        //unsafe { ::std::mem::transmute(self.cell.get()) }
    }
}
impl<T> DerefMut for JustOnce<T>
    where T: Send + Sync
{
    fn deref_mut(&mut self) -> &mut T {
        // `&mut self` means no JustOnceGuard's exist.
        debug_assert_ne!(self.atom.load(Ordering::SeqCst), *WAIT);
        unsafe { &mut *self.cell.get() }
        //unsafe { ::std::mem::transmute(self.cell.get()) }
    }
}

pub struct JustOnceGuard<'a, T: 'a> {
    mutex: &'a JustOnce<T>,
    _guard: MutexGuard<'a, ()>,
}

impl<'a, T> JustOnceGuard<'a, T>
    where T: 'a
{
    fn new(mutex: &'a JustOnce<T>) -> JustOnceGuard<'a, T> {
        let _guard = mutex.lock.lock();
        JustOnceGuard { mutex, _guard }
    }
}

impl<'a, T> ::std::fmt::Debug for JustOnceGuard<'a, T>
    where T: Send + Sync
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.pad("JustOnceGuard")
    }
}
impl<'a, T> Deref for JustOnceGuard<'a, T>
    where T: Send + Sync
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.cell.get() }
        //unsafe { ::std::mem::transmute(self.mutex.cell.get()) }
    }
}
impl<'a, T> DerefMut for JustOnceGuard<'a, T>
    where T: Send + Sync
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.cell.get() }
        // unsafe { ::std::mem::transmute(self.mutex.cell.get()) }
    }
}
impl<'a, T> Drop for JustOnceGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.atom.store(*FREE, Ordering::SeqCst);
    }
}

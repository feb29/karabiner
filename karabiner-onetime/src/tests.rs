use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

use super::*;

#[test]
fn lock_just_once() {
    let once = Lock::new("Lock");

    assert_eq!(once.atom.load(Ordering::SeqCst), *INIT);
    assert!(once.try_lock().is_some());

    assert_eq!(once.atom.load(Ordering::SeqCst), *FREE);
    assert!(once.try_lock().is_none());

    assert_eq!(once.atom.load(Ordering::SeqCst), *FREE);
    assert!(once.try_lock().is_none());
}

#[test]
fn just_once_atom() {
    let once1 = Arc::new(Lock::new("Lock"));

    let once2 = once1.clone();
    thread::spawn(move || {
                      let lock = once2.try_lock();
                      let stored = once2.atom.load(Ordering::SeqCst);
                      assert!(stored == *WAIT || stored == *FREE);
                      drop(lock)
                  });

    if let Some(data) = once1.try_lock() {
        println!("lock, {:?}", *data)
    } else {
        once1.wait();
        println!("wait, {:?}", **once1)
    }

    assert_eq!(once1.atom.load(Ordering::SeqCst), *FREE);
}

#[test]
fn just_once_deref() {
    let once = Lock::new("Lock");
    let deref = *once;
    assert_eq!(once.atom.load(Ordering::SeqCst), *FREE, "{:?}", deref);
}

#[test]
fn assert_send_sync() {
    fn __assert_send<T: Send>() {}
    fn __assert_sync<T: Sync>() {}
    __assert_send::<Lock<Vec<u8>>>();
    __assert_sync::<Lock<Vec<u8>>>();
}

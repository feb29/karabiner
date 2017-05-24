#![feature(test)]

extern crate karabiner_onetime;
extern crate test;
extern crate parking_lot;

use karabiner_onetime::Lock;
use test::Bencher;

#[bench]
fn just_once_lock(bencher: &mut Bencher) {
    let once = Lock::new(5);
    bencher.iter(|| { ::test::black_box(once.try_lock()); });
}

#[bench]
fn just_once_deref(bencher: &mut Bencher) {
    let once = Lock::new(5);
    bencher.iter(|| ::test::black_box(*once));
}

#[bench]
fn mutex_locking(bencher: &mut test::Bencher) {
    let mutex = parking_lot::Mutex::new(5);
    bencher.iter(|| ::test::black_box(mutex.lock()));
}

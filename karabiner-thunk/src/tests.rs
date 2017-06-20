use super::*;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn assert_send_sync() {
    fn __assert_send<T: Send>() {}
    fn __assert_sync<T: Sync>() {}
    __assert_send::<Thunk<Vec<u8>>>();
    __assert_sync::<Thunk<Vec<u8>>>();
}

#[test]
fn evaluate_just_once() {
    let c1 = Arc::new(Mutex::new(0));
    let c2 = c1.clone();

    let value = lazy!({
        let mut data = c1.lock().unwrap();
        *data += 1;
    });

    assert_eq!(*c2.lock().unwrap(), 0);
    *value;
    assert_eq!(*c2.lock().unwrap(), 1);
    *value;
    assert_eq!(*c2.lock().unwrap(), 1);
}

#[test]
fn print_once() {
    let expr = lazy!({
        println!("evaluated!");
        7
    });

    assert_eq!(*expr, 7); // "evaluated!" printed here.
    assert_eq!(*expr, 7); // Nothing printed.
    assert_eq!(*expr, 7);
}

#[test]
fn thunks_in_vec() {
    let arc0 = Arc::new(Mutex::new(Vec::new()));
    let arc1 = arc0.clone();
    let arc2 = arc0.clone();
    let arc3 = arc0.clone();
    let mut vec = vec![
        lazy!({
            arc1.lock().unwrap().push(0);
            0
        }),
        lazy!({
            arc2.lock().unwrap().push(1);
            1
        }),
        lazy!({
            arc3.lock().unwrap().push(2);
            2
        }),
        eval!(3),
    ];

    assert_eq!(vec.len(), 4);
    let removed = vec.remove(2);
    assert_eq!(vec.len(), 3);

    for thunk in vec.iter() {
        Thunk::force(thunk); // can't unwrap because unwrap need owenership.
    }

    {
        let locked = arc0.lock().unwrap();
        assert!(*locked == vec![0, 1], "{:?}", *locked);
    }
    assert!(*removed == 2); // removed thunk evaluate here.
    {
        let locked = arc0.lock().unwrap();
        assert!(*locked == vec![0, 1, 2], "{:?}", *locked);
    }
}

#[test]
fn evaluate_at_deref() {
    let value = lazy!(1000);
    assert_eq!(*value, 1000);
}

struct DropTest(Arc<Mutex<u64>>);
impl DropTest {
    fn value(&self) -> u64 {
        let DropTest(ref c) = *self;
        *c.lock().unwrap()
    }
}
impl Drop for DropTest {
    fn drop(&mut self) {
        let DropTest(ref c) = *self;
        *c.lock().unwrap() += 1;
        println!("drop {:?} ", c);
    }
}

#[test]
fn drop_just_once() {
    let c1 = Arc::new(Mutex::new(0));
    let c2 = c1.clone();

    let th = thread::spawn(move || {
        let drop = DropTest(c2);
        let lazy = lazy!({
            let drop_ref = &drop;
            assert!(drop_ref.value() == 0, "drop_ref:{:?}", drop_ref.value());
        });
        Thunk::force(&lazy);
    });

    match th.join() {
        Ok(_) => assert_eq!(*c1.lock().unwrap(), 1),
        Err(_) => unreachable!(),
    }
}

#[test]
fn thunk_in_thunk() {
    let t1 = lazy!({
        println!("t1");
        1 + 2
    });
    let t2 = lazy!({
        println!("t2");
        3 + 4
    });

    let t3 = lazy!({
        println!("evaluate thunk in thunk");
        let r1 = *t1;
        let r2 = *t2;
        (r1 + r2) * r1
    });

    assert!(*t3 == 30);
}

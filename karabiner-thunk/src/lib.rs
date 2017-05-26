#![feature(fnbox)]

extern crate karabiner_onetime;

use std::ops::{Deref, DerefMut};
use std::boxed::FnBox;

use karabiner_onetime::Lock;

#[macro_export]
macro_rules! lazy {
    ( $e:expr ) => {$crate::Thunk::lazy(move || { $e })};
}

#[macro_export]
macro_rules! eval {
    ( $e:expr ) => {$crate::Thunk::eval($e)};
}

#[cfg(test)]
mod tests;

pub struct Thunk<'a, T> {
    once: Lock<Expr<'a, T>>,
}
unsafe impl<'a, T: Send> Send for Thunk<'a, T> {}
unsafe impl<'a, T: Sync> Sync for Thunk<'a, T> {}

impl<'a, T: Send + Sync> Thunk<'a, T> {
    pub fn lazy<F>(f: F) -> Self
        where F: 'a + FnBox() -> T + Send + Sync
    {
        let expr = Expr::Deferred(Yield::new(f));
        let once = Lock::new(expr);
        Self { once }
    }

    pub fn eval(val: T) -> Thunk<'a, T> {
        let once = Lock::new(Expr::Evaluated(val));
        once.try_lock();
        Thunk { once }
    }

    pub fn force(thunk: &Self) {
        match thunk.once.try_lock() {
            Some(mut lock) => {
                match ::std::mem::replace(&mut *lock, Expr::InProgress) {
                    Expr::Deferred(f) => *lock = Expr::Evaluated(f.invoke()),
                    _ => unreachable!("Lock locked, but an inner expr is not defferred."),
                }
            }
            None => thunk.once.wait(),
        }
    }
}

impl<'a, T: Send + Sync> Deref for Thunk<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        Self::force(self);
        match *self.once {
            Expr::Evaluated(ref val) => val,
            _ => unreachable!("invoked force, but hold unevaluated value"),
        }
    }
}
impl<'a, T: Send + Sync> DerefMut for Thunk<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        Self::force(self);
        match *self.once {
            Expr::Evaluated(ref mut val) => unsafe { ::std::mem::transmute(val) },
            _ => unreachable!("invoked force, but hold unevaluated value"),
        }
    }
}


enum Expr<'a, T> {
    Deferred(Yield<'a, T>),
    InProgress,
    Evaluated(T),
}
unsafe impl<'a, T: Send> Send for Expr<'a, T> {}
unsafe impl<'a, T: Sync> Sync for Expr<'a, T> {}

struct Yield<'a, T> {
    boxed: Box<FnBox() -> T + 'a>,
}
impl<'a, T> Yield<'a, T> {
    fn new<F>(f: F) -> Yield<'a, T>
        where F: 'a + FnBox() -> T
    {
        let boxed = Box::new(f);
        Yield { boxed }
    }

    fn invoke(self) -> T {
        (self.boxed)()
    }
}

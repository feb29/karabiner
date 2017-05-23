pub extern crate karabiner_justonce as justonce;
pub extern crate karabiner_thunk as thunk;

// macros cannot reexoprt.

#[macro_export]
macro_rules! lazy {
    ( $e:expr ) => { $crate::thunk::Thunk::lazy(move || { $e }) };
}

#[macro_export]
macro_rules! eval {
    ( $e:expr ) => { $crate::thunk::Thunk::eval($e) };
}

pub use sim::{math, world};
#[path = "main.rs"]
#[allow(dead_code)]
mod runner;
use std::cell::RefCell;
thread_local! {static INPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};static OUTPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};}
#[no_mangle]
pub extern "C" fn reserve(n: usize) -> *mut u8 {
    INPUT.with(|v| {
        let mut v = v.borrow_mut();
        v.resize(n, 0);
        v.as_mut_ptr()
    })
}
#[no_mangle]
pub extern "C" fn run() -> usize {
    let records = INPUT.with(|v| runner::compute(serde_json::from_slice(&v.borrow()).unwrap()));
    OUTPUT.with(|v| {
        let mut v = v.borrow_mut();
        *v = serde_json::to_vec(&records).unwrap();
        v.len()
    })
}
#[no_mangle]
pub extern "C" fn result_ptr() -> *const u8 {
    OUTPUT.with(|v| v.borrow().as_ptr())
}

pub use sim::{math, world};
#[path = "main.rs"]
#[allow(dead_code)]
mod runner;
use std::cell::RefCell;
thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
// Single synchronous caller: reserve, fill input, run, read output before next call.
#[no_mangle]
pub extern "C" fn reserve(len: usize) -> *mut u8 {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        input.resize(len, 0);
        input.as_mut_ptr()
    })
}
#[no_mangle]
pub extern "C" fn run() -> usize {
    let records = INPUT.with(|input| {
        let rules: contract::scenario::Rules = serde_json::from_slice(&input.borrow()).unwrap();
        runner::compute_physical(&rules)
    });
    OUTPUT.with(|output| {
        let mut output = output.borrow_mut();
        *output = serde_json::to_vec(&records).unwrap();
        output.len()
    })
}
#[no_mangle]
pub extern "C" fn result_ptr() -> *const u8 {
    OUTPUT.with(|output| output.borrow().as_ptr())
}

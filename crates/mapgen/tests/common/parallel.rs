//! A sweep over independent maps, run side by side.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// `answer` for every case, on as many threads as the machine has cores,
/// in the order the cases are listed: a sweep reads the same whichever
/// finishes first. A panic in any case fails the caller.
pub fn each<T: Sync, R: Send>(cases: &[T], answer: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let answers: Mutex<Vec<Option<R>>> = Mutex::new((0..cases.len()).map(|_| None).collect());
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(cases.len());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(case) = cases.get(k) else { break };
                let a = answer(case);
                answers.lock().unwrap()[k] = Some(a);
            });
        }
    });
    answers
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|a| a.expect("every case answered"))
        .collect()
}

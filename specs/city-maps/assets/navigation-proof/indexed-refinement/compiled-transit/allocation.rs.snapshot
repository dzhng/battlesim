use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counter;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) {
    let live = CURRENT.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(l.size()) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let p = System.alloc(l);
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(l.size()) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        CURRENT.fetch_sub(l.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(n) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let q = System.realloc(p, l, n);
        if !q.is_null() {
            if n >= l.size() {
                add(n - l.size());
            } else {
                CURRENT.fetch_sub(l.size() - n, Ordering::Relaxed);
            }
        }
        q
    }
}
#[global_allocator]
static ALLOC: Counter = Counter;

pub fn bytes() -> (usize, usize) {
    (
        CURRENT.load(Ordering::Relaxed),
        PEAK.load(Ordering::Relaxed),
    )
}

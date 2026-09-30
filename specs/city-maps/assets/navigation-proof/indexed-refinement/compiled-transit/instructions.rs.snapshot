//! The reports' load-independent cost measure: unlike wall time, the
//! instructions a process retires do not move with machine load.

/// Instructions this process has retired, all its threads together, where
/// the OS counts them per process without privileges (macOS:
/// `proc_pid_rusage`); `None` elsewhere.
#[cfg(target_os = "macos")]
pub fn instructions() -> Option<u64> {
    extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
    }
    // `rusage_info_v4`: a 16-byte uuid, then 64-bit counters, the
    // instruction count the 30th of them (index 29).
    const RUSAGE_INFO_V4: i32 = 4;
    const INSTRUCTIONS: usize = 2 + 29;
    let mut info = [0u64; 64];
    // SAFETY: the buffer outlasts the call and is larger than
    // `rusage_info_v4`, the most the kernel writes for this flavor.
    let ok =
        unsafe { proc_pid_rusage(std::process::id() as i32, RUSAGE_INFO_V4, info.as_mut_ptr()) };
    (ok == 0).then_some(info[INSTRUCTIONS])
}

#[cfg(not(target_os = "macos"))]
pub fn instructions() -> Option<u64> {
    None
}

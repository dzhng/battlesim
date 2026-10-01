//! The reports' load-independent cost measure: unlike wall time, the
//! instructions a process retires do not move with machine load.

/// Process instructions and CPU nanoseconds, across all its threads, from
/// macOS `proc_pid_rusage` without privileges; `None` elsewhere.
#[cfg(target_os = "macos")]
pub fn resources() -> Option<(u64, u64)> {
    extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
        fn mach_timebase_info(info: *mut u32) -> i32;
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
    if ok != 0 {
        return None;
    }
    static TIMEBASE: std::sync::OnceLock<[u32; 2]> = std::sync::OnceLock::new();
    let [numer, denom] = *TIMEBASE.get_or_init(|| {
        let mut ratio = [0; 2];
        // SAFETY: mach_timebase_info writes two u32 fields to this live buffer.
        assert_eq!(unsafe { mach_timebase_info(ratio.as_mut_ptr()) }, 0);
        ratio
    });
    // The kernel stores these durations in Mach absolute-time ticks.
    let cpu_ns = ((info[2] as u128 + info[3] as u128) * numer as u128 / denom as u128) as u64;
    Some((info[INSTRUCTIONS], cpu_ns))
}

#[cfg(not(target_os = "macos"))]
pub fn resources() -> Option<(u64, u64)> {
    None
}

/// The count alone, for reports that do not need the process's CPU nanoseconds.
pub fn instructions() -> Option<u64> {
    resources().map(|(count, _)| count)
}

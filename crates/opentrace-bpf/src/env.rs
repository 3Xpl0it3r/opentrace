// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
pub fn setup_memlock_limit() {
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        eprintln!("Warning: Failed to remove memory lock limit (ret={})", ret);
    }
}

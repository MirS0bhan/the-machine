//! Example Lambda function that attempts a forbidden syscall.
//!
//! When sandboxed with CAP_PURE, `open(2)` is NOT in the seccomp allowlist, so
//! the kernel kills this process before it can do anything. The Lambda Server
//! reports the invocation as killed-by-seccomp — proving the whitelist works.

use std::ffi::CString;

fn main() {
    let path = CString::new("/etc/passwd").unwrap();
    // open(2) is denied for a pure function -> seccomp KILL_PROCESS.
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
    // If we somehow reach here, the sandbox failed.
    println!("{{\"escaped\": true, \"fd\": {}}}", fd);
}

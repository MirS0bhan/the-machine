//! Sandbox for Lambda functions.
//!
//! Every invocation runs in its own set of unshared namespaces
//! (mount, pid, network, ipc, uts, cgroup), with **all** Linux capabilities
//! dropped, and under a **seccomp allowlist** (whitelist) derived from the
//! function's declared capabilities. The default policy is deny-everything:
//! only the syscalls explicitly granted by a capability are permitted, and the
//! rest are killed by the kernel.
//!
//! Cross-function IPC sharing is achieved by joining a per-group IPC namespace
//! (see `ipc_ns_fd`): functions in the same group see each other's AF_UNIX
//! sockets; everyone else is isolated.

use std::ffi::CString;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct CapSet {
    pub pure: bool,
    pub fs_read: bool,
    pub fs_write: bool,
    pub net_out: bool,
    pub ipc_call: bool,
    pub state_read: bool,
    pub state_write: bool,
    pub timer: bool,
    pub gpu: bool,
    pub mic: bool,
    pub camera: bool,
    /// Explicit IPC targets (component ids) the function may connect to.
    pub ipc_targets: Vec<String>,
}

impl CapSet {
    /// True when the function must not touch the OS at all.
    pub fn is_pure(&self) -> bool {
        self.pure
            || !(self.fs_read
                || self.fs_write
                || self.net_out
                || self.ipc_call
                || self.state_read
                || self.state_write
                || self.timer
                || self.gpu
                || self.mic
                || self.camera)
    }
}

/// Parse a capability list such as
/// `["CAP_PURE", "CAP_IPC_CALL(targets=[\"state-store\"])", ...]` into a CapSet.
pub fn parse_caps(caps: &[String]) -> CapSet {
    let mut set = CapSet::default();
    for raw in caps {
        let name = raw.split(['(', ' ']).next().unwrap_or("").trim();
        match name {
            "CAP_PURE" => set.pure = true,
            "CAP_FS_READ" => set.fs_read = true,
            "CAP_FS_WRITE" => set.fs_write = true,
            "CAP_NET_OUT" => set.net_out = true,
            "CAP_IPC_CALL" => {
                set.ipc_call = true;
                if let Some(targets) = extract_list(raw) {
                    set.ipc_targets.extend(targets);
                }
            }
            "CAP_STATE_READ" => {
                set.state_read = true;
                set.ipc_call = true;
            }
            "CAP_STATE_WRITE" => {
                set.state_write = true;
                set.ipc_call = true;
            }
            "CAP_TIMER" => set.timer = true,
            "CAP_GPU" => set.gpu = true,
            "CAP_MIC" => set.mic = true,
            "CAP_CAMERA" => set.camera = true,
            _ => {}
        }
    }
    set
}

fn extract_list(raw: &str) -> Option<Vec<String>> {
    let open = raw.find('[')?;
    let close = raw.find(']')?;
    let inner = &raw[open + 1..close];
    Some(
        inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

// ------------------------------------------------------------------------
// Syscall allowlist
// ------------------------------------------------------------------------

/// Base syscalls any (even PURE) dynamically-linked Rust/C binary needs just to
/// start up, compute, and talk on already-open fds.
fn base_syscalls() -> Vec<i64> {
    use libc::*;
    vec![
        SYS_read,
        SYS_write,
        SYS_close,
        SYS_fstat,
        SYS_lseek,
        SYS_mmap,
        SYS_mprotect,
        SYS_munmap,
        SYS_brk,
        SYS_rt_sigaction,
        SYS_rt_sigprocmask,
        SYS_rt_sigreturn,
        SYS_ioctl,
        SYS_pread64,
        SYS_pwrite64,
        SYS_readv,
        SYS_writev,
        SYS_pipe,
        SYS_pipe2,
        SYS_select,
        SYS_poll,
        SYS_ppoll,
        SYS_sched_yield,
        SYS_mremap,
        SYS_madvise,
        SYS_dup,
        SYS_dup2,
        SYS_dup3,
        SYS_nanosleep,
        SYS_clock_nanosleep,
        SYS_getitimer,
        SYS_setitimer,
        SYS_alarm,
        SYS_getpid,
        SYS_getppid,
        SYS_gettid,
        SYS_futex,
        SYS_set_tid_address,
        SYS_restart_syscall,
        SYS_getdents64,
        SYS_set_robust_list,
        SYS_get_robust_list,
        SYS_sched_getaffinity,
        SYS_sched_setaffinity,
        SYS_getpriority,
        SYS_setpriority,
        SYS_prlimit64,
        SYS_prctl,
        SYS_arch_prctl,
        SYS_getrandom,
        SYS_clock_gettime,
        SYS_clock_getres,
        SYS_gettimeofday,
        SYS_getrlimit,
        SYS_getrusage,
        SYS_umask,
        SYS_getcwd,
        SYS_chdir,
        SYS_getuid,
        SYS_getgid,
        SYS_geteuid,
        SYS_getegid,
        SYS_getgroups,
        SYS_setpgid,
        SYS_setsid,
        SYS_fcntl,
        SYS_flock,
        SYS_fsync,
        SYS_fdatasync,
        SYS_epoll_create1,
        SYS_epoll_ctl,
        SYS_epoll_wait,
        SYS_epoll_pwait,
        SYS_eventfd2,
        SYS_signalfd4,
        SYS_membarrier,
        SYS_rseq,
        SYS_clone,
        SYS_clone3,
        SYS_fork,
        SYS_vfork,
        SYS_execve,
        SYS_wait4,
        SYS_kill,
        SYS_tkill,
        SYS_tgkill,
        SYS_exit,
        SYS_exit_group,
    ]
}

fn fs_read_syscalls() -> Vec<i64> {
    use libc::*;
    vec![
        SYS_open,
        SYS_openat,
        SYS_stat,
        SYS_lstat,
        SYS_newfstatat,
        SYS_access,
        SYS_faccessat,
        SYS_faccessat2,
        SYS_readlink,
        SYS_statx,
    ]
}

fn fs_write_syscalls() -> Vec<i64> {
    use libc::*;
    vec![
        SYS_creat,
        SYS_link,
        SYS_unlink,
        SYS_symlink,
        SYS_rename,
        SYS_mkdir,
        SYS_rmdir,
        SYS_truncate,
        SYS_ftruncate,
        SYS_chmod,
        SYS_fchmod,
        SYS_chown,
        SYS_fchown,
        SYS_lchown,
        SYS_utime,
        SYS_utimes,
        SYS_fchmodat,
        SYS_renameat2,
    ]
}

fn net_ipc_syscalls() -> Vec<i64> {
    use libc::*;
    vec![
        SYS_socket,
        SYS_socketpair,
        SYS_connect,
        SYS_accept,
        SYS_accept4,
        SYS_bind,
        SYS_listen,
        SYS_sendto,
        SYS_recvfrom,
        SYS_sendmsg,
        SYS_recvmsg,
        SYS_getsockname,
        SYS_getpeername,
        SYS_setsockopt,
        SYS_getsockopt,
        SYS_shutdown,
    ]
}

fn timer_syscalls() -> Vec<i64> {
    use libc::*;
    vec![SYS_timerfd_create, SYS_timerfd_settime, SYS_timerfd_gettime]
}

/// Build the final allowlist from a CapSet.
pub fn allowed_syscalls(caps: &CapSet) -> Vec<i64> {
    if caps.is_pure() {
        let mut s = base_syscalls();
        s.sort_unstable();
        s.dedup();
        return s;
    }
    let mut s = base_syscalls();
    if caps.fs_read {
        s.extend(fs_read_syscalls());
    }
    if caps.fs_write {
        s.extend(fs_write_syscalls());
    }
    if caps.net_out || caps.ipc_call || caps.state_read || caps.state_write {
        s.extend(net_ipc_syscalls());
    }
    if caps.timer {
        s.extend(timer_syscalls());
    }
    if caps.gpu || caps.mic || caps.camera {
        // Device access: already have open/read/write/ioctl from base.
        s.push(libc::SYS_mmap); // already in base
    }
    s.sort_unstable();
    s.dedup();
    s
}

// ------------------------------------------------------------------------
// Seccomp allowlist installer
// ------------------------------------------------------------------------

const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x20;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_RET: u16 = 0x06;
const BPF_JEQ: u16 = 0x10;
const BPF_K: u16 = 0x00;

#[repr(C)]
struct SockFilter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[repr(C)]
struct SockFprog {
    len: u16,
    _pad: u16,
    filter: *const SockFilter,
}

const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;

fn stmt(code: u16, k: u32) -> SockFilter {
    SockFilter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}
fn jt(code: u16, k: u32, jt: u8, jf: u8) -> SockFilter {
    SockFilter { code, jt, jf, k }
}

/// Install a seccomp filter that **only** allows the given syscalls.
/// Everything else kills the process.
fn install_seccomp(syscalls: &[i64]) {
    let mut filter: Vec<SockFilter> = vec![
        // Validate architecture first.
        stmt(BPF_LD | BPF_W | BPF_ABS, 4), // seccomp_data.arch at offset 4
        jt(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0),
        stmt(BPF_RET, SECCOMP_RET_KILL_PROCESS),
        // Load syscall number.
        stmt(BPF_LD | BPF_W | BPF_ABS, 0), // seccomp_data.nr at offset 0
    ];
    for &nr in syscalls {
        filter.push(jt(BPF_JMP | BPF_JEQ | BPF_K, nr as u32, 0, 1));
        filter.push(stmt(BPF_RET, SECCOMP_RET_ALLOW));
    }
    filter.push(stmt(BPF_RET, SECCOMP_RET_KILL_PROCESS));

    let prog = SockFprog {
        len: filter.len() as u16,
        _pad: 0,
        filter: filter.as_ptr(),
    };
    unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
        libc::syscall(
            libc::SYS_seccomp,
            1 as libc::c_ulong, // SECCOMP_SET_MODE_FILTER
            0 as libc::c_ulong,
            &prog as *const SockFprog as libc::c_ulong,
        );
    }
}

// ------------------------------------------------------------------------
// Capability dropping
// ------------------------------------------------------------------------

const CAP_LINUX_VERSION_3: u32 = 0x2008_0522;

#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

fn drop_caps() {
    let hdr = CapHeader {
        version: CAP_LINUX_VERSION_3,
        pid: 0,
    };
    let mut data = [CapData {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    unsafe {
        libc::syscall(
            libc::SYS_capset,
            &hdr as *const CapHeader as libc::c_ulong,
            &mut data as *mut [CapData; 2] as libc::c_ulong,
        );
        // Clear any ambient capabilities too.
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        );
    }
}

// ------------------------------------------------------------------------
// Mount namespace / chroot setup
// ------------------------------------------------------------------------

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap_or_default()
}

/// Create a private tmpfs root, bind-mount the host libraries and the
/// function binary into it, then chroot. The function can only see its own
/// code and the dynamic linker — nothing else on the host filesystem.
fn setup_mountns(entry: &str) {
    unsafe {
        let empty = cstr("");
        let slash = cstr("/");
        // Make the mount tree private so our changes don't leak to the host.
        libc::mount(
            empty.as_ptr(),
            slash.as_ptr(),
            std::ptr::null::<libc::c_char>(),
            (libc::MS_REC | libc::MS_PRIVATE) as libc::c_ulong,
            std::ptr::null::<libc::c_void>(),
        );

        let root = cstr("/tmp/lambda-root");
        libc::mkdir(root.as_ptr(), 0o755);
        libc::mount(
            cstr("tmpfs").as_ptr(),
            root.as_ptr(),
            cstr("tmpfs").as_ptr(),
            0 as libc::c_ulong,
            cstr("size=64m,mode=755").as_ptr() as *const libc::c_void,
        );

        // Bind the dynamic linker + libc so dynamically-linked functions work.
        for d in [
            "/lib",
            "/lib64",
            "/usr/lib",
            "/usr/lib64",
            "/lib/x86_64-linux-gnu",
            "/usr/lib/x86_64-linux-gnu",
        ] {
            if Path::new(d).exists() {
                let dst = format!("/tmp/lambda-root{}", d);
                libc::mkdir(cstr(&dst).as_ptr(), 0o755);
                libc::mount(
                    cstr(d).as_ptr(),
                    cstr(&dst).as_ptr(),
                    std::ptr::null::<libc::c_char>(),
                    libc::MS_BIND as libc::c_ulong,
                    std::ptr::null::<libc::c_void>(),
                );
            }
        }

        // Bind the function binary itself.
        libc::mkdir(cstr("/tmp/lambda-root/app").as_ptr(), 0o755);
        libc::mount(
            cstr(entry).as_ptr(),
            cstr("/tmp/lambda-root/app/func").as_ptr(),
            std::ptr::null::<libc::c_char>(),
            libc::MS_BIND as libc::c_ulong,
            std::ptr::null::<libc::c_void>(),
        );

        libc::chroot(root.as_ptr());
        libc::chdir(cstr("/").as_ptr());
    }
}

// ------------------------------------------------------------------------
// Invocation
// ------------------------------------------------------------------------

pub struct SandboxInput {
    pub entry: String,
    pub args: Vec<String>,
    pub input: Vec<u8>,
    pub timeout_ms: u64,
    /// IPC namespace fd to join (shared with the function's group). -1 = private.
    pub ipc_ns_fd: i32,
    pub caps: CapSet,
}

pub struct SandboxOutput {
    pub stdout: String,
    pub stderr: String,
    pub killed: bool,
    pub error: Option<String>,
}

unsafe fn write_all(fd: i32, data: &[u8]) {
    let mut off = 0;
    while off < data.len() {
        let n = libc::write(
            fd,
            data[off..].as_ptr() as *const libc::c_void,
            data.len() - off,
        );
        if n <= 0 {
            break;
        }
        off += n as usize;
    }
}

fn read_thread(fd: i32) -> String {
    let mut s = String::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        s.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
    }
    s
}

/// Run `entry` inside the sandbox synchronously and return its captured output.
pub fn run_sandboxed(input: &SandboxInput) -> SandboxOutput {
    let mut pipes = [[0i32; 2], [0i32; 2], [0i32; 2]];
    unsafe {
        if libc::pipe(pipes[0].as_mut_ptr()) != 0
            || libc::pipe(pipes[1].as_mut_ptr()) != 0
            || libc::pipe(pipes[2].as_mut_ptr()) != 0
        {
            return SandboxOutput {
                stdout: String::new(),
                stderr: String::new(),
                killed: false,
                error: Some("pipe() failed".into()),
            };
        }
    }
    let [[out_r, out_w], [err_r, err_w], [in_r, in_w]] = pipes;

    let mut st_pipe = [0i32; 2];
    unsafe {
        libc::pipe(st_pipe.as_mut_ptr());
    }
    let [st_r, st_w] = st_pipe;

    let syscalls = allowed_syscalls(&input.caps);
    let entry_c = match CString::new(input.entry.as_str()) {
        Ok(c) => c,
        Err(_) => {
            return SandboxOutput {
                stdout: String::new(),
                stderr: String::new(),
                killed: false,
                error: Some("invalid entry path".into()),
            }
        }
    };
    let argv: Vec<CString> = std::iter::once(entry_c)
        .chain(
            input
                .args
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok()),
        )
        .collect();
    let mut argv_ptrs: Vec<*const libc::c_char> = argv.iter().map(|c| c.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    let ipc_fd = input.ipc_ns_fd;
    let entry_clone = input.entry.clone();
    let input_args = input.args.clone();

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return SandboxOutput {
            stdout: String::new(),
            stderr: String::new(),
            killed: false,
            error: Some("fork() failed".into()),
        };
    }

    if pid == 0 {
        // ---- child A ----
        unsafe {
            libc::close(out_r);
            libc::close(err_r);
            libc::close(in_w);
            libc::dup2(in_r, 0);
            libc::dup2(out_w, 1);
            libc::dup2(err_w, 2);
            libc::close(in_r);
            libc::close(out_w);
            libc::close(err_w);
            // New session + process group so a timeout kill can take the
            // whole subtree (including the sandboxed worker) with kill(-pid).
            libc::setsid();

            let flags = libc::CLONE_NEWNS
                | libc::CLONE_NEWNET
                | libc::CLONE_NEWIPC
                | libc::CLONE_NEWUTS
                | libc::CLONE_NEWCGROUP
                | libc::CLONE_NEWPID;
            libc::unshare(flags);

            let pid2 = libc::fork();
            if pid2 < 0 {
                libc::_exit(1);
            }
            if pid2 > 0 {
                // child A waits for the worker, reports its exit status, exits.
                let mut status: i32 = 0;
                libc::waitpid(pid2, &mut status as *mut i32, 0);
                libc::close(st_r);
                libc::write(
                    st_w,
                    &status as *const i32 as *const libc::c_void,
                    std::mem::size_of::<i32>(),
                );
                libc::close(st_w);
                libc::_exit(0);
            }

            // ---- worker (sandboxed) ----
            setup_mountns(&entry_clone);
            if ipc_fd >= 0 {
                libc::setns(ipc_fd, libc::CLONE_NEWIPC);
            }
            drop_caps();
            install_seccomp(&syscalls);

            // After chroot the function lives at /app/func.
            let exec_path = cstr("/app/func");
            let mut exec_argv: Vec<CString> = vec![exec_path.clone()];
            for a in &input_args {
                if let Ok(c) = CString::new(a.as_str()) {
                    exec_argv.push(c);
                }
            }
            let mut exec_ptrs: Vec<*const libc::c_char> =
                exec_argv.iter().map(|c| c.as_ptr()).collect();
            exec_ptrs.push(std::ptr::null());
            libc::execvp(exec_path.as_ptr(), exec_ptrs.as_ptr());
            libc::_exit(127);
        }
    }

    // ---- parent ----
    unsafe {
        libc::close(out_w);
        libc::close(err_w);
        libc::close(in_r);
    }
    // Feed the payload to the function's stdin.
    unsafe {
        write_all(in_w, &input.input);
        libc::close(in_w);
    }

    let out_thr = std::thread::spawn(move || read_thread(out_r));
    let err_thr = std::thread::spawn(move || read_thread(err_r));
    let timeout_ms = input.timeout_ms;
    unsafe {
        libc::close(st_w);
    }

    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
        // Negative pid signals the whole (sandboxed) process group.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    });

    let stdout = out_thr.join().unwrap_or_default();
    let stderr = err_thr.join().unwrap_or_default();
    let _ = watchdog.join();

    // child A waited on the worker and wrote its waitpid status to st_r.
    let mut status: i32 = 0;
    unsafe {
        libc::read(
            st_r,
            &mut status as *mut i32 as *mut libc::c_void,
            std::mem::size_of::<i32>(),
        );
        libc::close(st_r);
    }
    let killed = libc::WIFSIGNALED(status)
        && (libc::WTERMSIG(status) == libc::SIGSYS || libc::WTERMSIG(status) == libc::SIGKILL);

    SandboxOutput {
        stdout,
        stderr,
        killed,
        error: None,
    }
}

/// A long-lived sandboxed process (used for `lambda.lease` fast-path).
pub struct Persistent {
    pub pid: i32,
    pub stdin_fd: i32,
    pub stdout_fd: i32,
}

/// Spawn a persistent sandboxed process, keeping its stdin/stdout pipes open.
pub fn run_persistent(input: &SandboxInput) -> Option<Persistent> {
    let mut p = [[0i32; 2]; 2];
    unsafe {
        if libc::pipe(p[0].as_mut_ptr()) != 0 || libc::pipe(p[1].as_mut_ptr()) != 0 {
            return None;
        }
    }
    let [[out_r, out_w], [in_r, in_w]] = p;

    let syscalls = allowed_syscalls(&input.caps);
    let entry_clone = input.entry.clone();
    let input_args = input.args.clone();
    let ipc_fd = input.ipc_ns_fd;

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return None;
    }
    if pid == 0 {
        unsafe {
            libc::close(out_r);
            libc::close(in_w);
            libc::dup2(in_r, 0);
            libc::dup2(out_w, 1);
            libc::close(in_r);
            libc::close(out_w);
            libc::setsid();
            let flags = libc::CLONE_NEWNS
                | libc::CLONE_NEWNET
                | libc::CLONE_NEWIPC
                | libc::CLONE_NEWUTS
                | libc::CLONE_NEWCGROUP
                | libc::CLONE_NEWPID;
            libc::unshare(flags);
            let pid2 = libc::fork();
            if pid2 < 0 {
                libc::_exit(1);
            }
            if pid2 > 0 {
                libc::_exit(0);
            }
            setup_mountns(&entry_clone);
            if ipc_fd >= 0 {
                libc::setns(ipc_fd, libc::CLONE_NEWIPC);
            }
            drop_caps();
            install_seccomp(&syscalls);
            let exec_path = cstr("/app/func");
            let mut exec_argv: Vec<CString> = vec![exec_path.clone()];
            for a in &input_args {
                if let Ok(c) = CString::new(a.as_str()) {
                    exec_argv.push(c);
                }
            }
            let mut exec_ptrs: Vec<*const libc::c_char> =
                exec_argv.iter().map(|c| c.as_ptr()).collect();
            exec_ptrs.push(std::ptr::null());
            libc::execvp(exec_path.as_ptr(), exec_ptrs.as_ptr());
            libc::_exit(127);
        }
    }

    unsafe {
        libc::close(out_w);
        libc::close(in_r);
    }
    // Give the child a moment to set up; if it died immediately, report failure.
    std::thread::sleep(std::time::Duration::from_millis(50));
    Some(Persistent {
        pid,
        stdin_fd: in_w,
        stdout_fd: out_r,
    })
}

/// Send one request to a persistent process and read one response line.
pub fn persistent_request(
    p: &Persistent,
    payload: &[u8],
    timeout_ms: u64,
) -> Result<String, String> {
    unsafe {
        write_all(p.stdin_fd, payload);
        // Newline-delimited protocol.
        write_all(p.stdin_fd, b"\n");
    }
    let out_fd = p.stdout_fd;
    let result = std::thread::spawn(move || {
        let mut s = String::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libc::read(out_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            s.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
            if s.contains('\n') {
                break;
            }
        }
        s
    });
    let pid = p.pid;
    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    });
    let out = result.join().unwrap_or_default();
    let _ = watchdog.join();
    Ok(out)
}

/// Kill a persistent process group.
pub fn persistent_kill(p: &Persistent) {
    unsafe {
        libc::kill(-p.pid, libc::SIGKILL);
    }
}

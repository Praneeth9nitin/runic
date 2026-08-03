use libseccomp::{ScmpAction, ScmpFilterContext, ScmpSyscall, ScmpArch};
use caps::{CapSet, Capability};

pub fn apply_seccomp() -> anyhow::Result<()>{
    let mut filter = ScmpFilterContext::new(ScmpAction::KillProcess)?;
    filter.add_arch(ScmpArch::X8664)?;
     let sys_call = [
        // File ops
        "read", "write", "open", "close", "stat", "fstat", "lstat",
        "openat", "readv", "writev", "pread64", "pwrite64",
        // Process
        "fork", "clone", "execve", "exit", "exit_group", "wait4",
        "getpid", "getppid", "gettid",
        // Memory
        "mmap", "mprotect", "munmap", "brk", "mremap",
        // Network
        "socket", "connect", "accept", "bind", "listen",
        "send", "recv", "sendto", "recvfrom",
        // Signals
        "rt_sigaction", "rt_sigprocmask", "rt_sigreturn", "kill", "tgkill",
        // Time
        "clock_gettime", "gettimeofday", "nanosleep",
        // Misc
        "ioctl", "fcntl", "dup", "dup2", "pipe", "getcwd", "chdir", "getdents64",
        "futex",           // thread synchronization — bash needs this
        "set_tid_address", // thread setup
        "set_robust_list", // thread setup
        "arch_prctl",      // thread-local storage
        "access",          // file permission checks
        "newfstatat",      // modern stat variant
        "getuid", "getgid", "geteuid", "getegid",  // user/group IDs
        "uname",           // system info
        "sysinfo",         // system statistics
        "prctl",           // process control
        "capget", "capset", // capabilities
        "sigaltstack",     // signal stack
        "madvise",         // memory hints
        "poll", "select",  // I/O multiplexing
        "lseek",           // file seeking
        "unlink", "mkdir", "rmdir", "rename", // filesystem ops
        "chmod", "chown",  // permission changes
    ];

    for i in sys_call{
        filter.add_rule(ScmpAction::Allow, ScmpSyscall::from_name(i).expect("msg"))?;
    }

    filter.load()?;
    Ok(())
}

pub fn drop_capabilities()-> anyhow::Result<()>{
    let to_drop = [
        Capability::CAP_SYS_ADMIN,
        Capability::CAP_SYS_PTRACE,
        Capability::CAP_SYS_BOOT,
        Capability::CAP_SYS_MODULE,
        Capability::CAP_SYS_RAWIO,
        Capability::CAP_SYS_BOOT,
        Capability::CAP_NET_ADMIN,
        Capability::CAP_NET_RAW,
        Capability::CAP_MKNOD,
        Capability::CAP_AUDIT_WRITE,
        Capability::CAP_AUDIT_CONTROL,
        Capability::CAP_SYSLOG,
        Capability::CAP_MAC_ADMIN,
        Capability::CAP_MAC_OVERRIDE,
        Capability::CAP_WAKE_ALARM,
        Capability::CAP_BLOCK_SUSPEND,
        Capability::CAP_LEASE,
    ];

    for cap in &to_drop{
        caps::drop(None, CapSet::Effective, *cap).ok();
        caps::drop(None, CapSet::Permitted, *cap).ok();
        caps::drop(None, CapSet::Inheritable, *cap).ok();
    }
    Ok(())
}
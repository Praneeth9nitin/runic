use libseccomp::{ScmpAction, ScmpFilterContext, ScmpSyscall, ScmpArch};
use caps::{CapSet, Capability};

pub fn apply_seccomp() -> anyhow::Result<()>{
    let mut filter = ScmpFilterContext::new(ScmpAction::Errno(1))?;
    println!("process killed");
    filter.add_arch(ScmpArch::X8664)?;
    let sys_call = [
        // File ops
        "read", "write", "open", "close", "stat", "fstat", "lstat",
        "openat", "readv", "writev", "pread64", "pwrite64",
        "access", "lseek", "fstatfs", "statx", "getdents64",
        "fcntl", "dup", "dup2", "pipe", "getcwd", "chdir",
        "newfstatat", "readlink", "readlinkat",  // ← critical for dynamic linker

        // Process
        "fork", "clone", "clone3", "execve", "exit", "exit_group",
        "wait4", "waitid",
        "getpid", "getppid", "gettid", "geteuid", "getuid",
        "getegid", "getgid", "getgroups", "getsid",
        "setgroups", "setresgid", "setresuid",  // ← user/group management

        // Memory
        "mmap", "mprotect", "munmap", "brk", "mremap", "madvise",

        // Network
        "socket", "connect", "accept", "accept4", "bind", "listen",
        "send", "recv", "sendto", "recvfrom", "sendmsg", "recvmsg",
        "getsockopt", "setsockopt", "getsockname", "getpeername",
        "shutdown", "socketpair",

        // Signals
        "rt_sigaction", "rt_sigprocmask", "rt_sigreturn",
        "kill", "tgkill", "sigaltstack",

        // Time
        "clock_gettime", "gettimeofday", "nanosleep",
        "timerfd_create", "timerfd_settime", "timerfd_gettime",

        // Thread/sync
        "futex", "set_tid_address", "set_robust_list", "get_robust_list",
        "sched_getaffinity",

        // Epoll
        "epoll_create1", "epoll_ctl", "epoll_wait", "epoll_pwait2",

        // System
        "arch_prctl", "prctl", "uname", "getrandom",
        "prlimit64", "rseq", "sysinfo",

        // Misc
        "ioctl", "poll", "ppoll", "select", "pselect6",
        "fsync", "fdatasync",
        "dup3", "pipe2",
        "setns", "unshare",

        // Resource limits
        "getrlimit", "setrlimit",
        "getrusage",

        // File management
        "unlink", "unlinkat", "mkdir", "mkdirat",
        "rmdir", "rename", "renameat", "renameat2",
        "chmod", "fchmod", "chown", "fchown", "lchown",
        "symlink", "symlinkat", "link", "linkat",
        "umask", "truncate", "ftruncate",

        // Misc system
        "memfd_create", "eventfd2", "signalfd4",
        "copy_file_range", "sendfile",
        "capget", "capset",
        "chroot", "pivot_root",
        "mount", "umount2",
        "sync", "syncfs",
        "getpgrp",
        "getpgid", 
        "setpgid",
        "setsid",
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
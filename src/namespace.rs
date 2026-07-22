use nix::sched::{CloneFlags, unshare};
use nix::unistd::sethostname;

pub fn set_namespace(hostname:String) -> anyhow::Result<()> {
    println!("setting up namespaces...");
    unshare(CloneFlags::CLONE_NEWUTS)
        .map_err(|e| anyhow::anyhow!("CLONE_NEWUTS failed: {}", e))?;
    println!("hostname set");
    unshare(CloneFlags::CLONE_NEWNS)
        .map_err(|e| anyhow::anyhow!("CLONE_NEWUTS failed: {}", e))?;
    println!("newns set");
    sethostname(format!("runic-{}",hostname))?;
    println!("hostname changed");
    // unshare(CloneFlags::CLONE_NEWPID)?;
    unshare(CloneFlags::CLONE_NEWNET)?;
    println!("namespaces done");
    Ok(())
}

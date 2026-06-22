use nix::sched::{CloneFlags, unshare};
use nix::unistd::sethostname;

pub fn set_namespace() -> anyhow::Result<()> {
    println!("setting up namespaces...");
    unshare(CloneFlags::CLONE_NEWUTS)?;
    unshare(CloneFlags::CLONE_NEWNS)?;
    sethostname("runic-container")?;
    println!("namespaces done");
    Ok(())
}

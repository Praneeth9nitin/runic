#[cfg(target_os = "linux")]
use nix::sched::{CloneFlags, unshare};
use nix::unistd::sethostname;

pub fn set_namespace() -> anyhow::Result<()> {
    println!("setting up namespaces...");
    #[cfg(target_os = "linux")]
    {
        unshare(CloneFlags::CLONE_NEWUTS)?;
    }
    sethostname("runic-container")?;
    println!("namespaces done");
    Ok(())
}

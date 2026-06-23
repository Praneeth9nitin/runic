use nix::mount::{mount, MsFlags, umount2, MntFlags};
use std::fs::create_dir_all;
use nix::unistd::pivot_root;

pub fn set_filesystem(container_id: &str) -> anyhow::Result<()>{
    mount(None::<&str>, "/", None::<&str>, MsFlags::MS_REC | MsFlags::MS_PRIVATE, None::<&str>)?;
    create_dir_all(format!("/tmp/runic/{}/upper", container_id))?;
    create_dir_all(format!("/tmp/runic/{}/work", container_id))?;
    create_dir_all(format!("/tmp/runic/{}/merged", container_id))?;
    create_dir_all(format!("/tmp/runic/{}/merged/oldroot", container_id))?;
    let target = format!("/tmp/runic/{}/merged",container_id);
    let old_root = format!("/tmp/runic/{}/merged/oldroot", container_id);

    let options = format!("lowerdir=/tmp/runic/base,upperdir=/tmp/runic/{}/upper,workdir=/tmp/runic/{}/work", container_id, container_id);
    mount(Some("overlay"), target.as_str(), Some("overlay"), MsFlags::empty(), Some(options.as_str()))?;
    println!("mount 1 done");
    mount(Some(target.as_str()), target.as_str(), None::<&str>, MsFlags::MS_BIND | MsFlags::MS_REC, None::<&str>)?;
    println!("mount 2 done");
    std::env::set_current_dir(&target)?; 
    println!("chdir done");
    pivot_root(".", old_root.as_str())?;
    println!("pivot done");
    std::env::set_current_dir("/")?;
    umount2("/oldroot", MntFlags::MNT_DETACH)?;
    println!("Unmount done");
    mount(Some("proc"), "/proc", Some("proc"), MsFlags::empty(), None::<&str>)?;
    println!("mount 3 done");
    Ok(())
}
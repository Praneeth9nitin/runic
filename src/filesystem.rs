use nix::mount::{mount, MsFlags, umount2, MntFlags};
use std::fs::create_dir_all;
use nix::unistd::pivot_root;

pub fn set_filesystem(container_id: &str) -> anyhow::Result<()>{
    create_dir_all(format!("/home/ubuntu/runic/rootfs/{}/upper", container_id))?;
    create_dir_all(format!("/home/ubuntu/runic/rootfs/{}/work", container_id))?;
    create_dir_all(format!("/home/ubuntu/runic/rootfs/{}/merged", container_id))?;
    create_dir_all(format!("/home/ubuntu/runic/rootfs/{}/merged/oldroot", container_id))?;
    let target = format!("/home/ubuntu/runic/rootfs/{}/merged",container_id);
    let old_root = format!("/home/ubuntu/runic/rootfs/{}/merged/oldroot", container_id);

     let options = format!("lowerdir=/home/ubuntu/runic/rootfs/base,\
                   upperdir=/home/ubuntu/runic/rootfs/{}/upper,
                   workdir=/home/ubuntu/runic/rootfs/{}/work", container_id, container_id);
    mount(Some("overlay"), target.as_str(), Some("overlay"), MsFlags::empty(), Some(options.as_str()))?;
    println!("mount 1 done");
    pivot_root(target.as_str(), old_root.as_str())?;
    println!("pivot done");
    umount2("/oldroot", MntFlags::MNT_DETACH)?;
    println!("Unmount done");
    mount(Some("proc"), "/proc", Some("proc"), MsFlags::empty(), None::<&str>)?;
    println!("mount 2 done");
    Ok(())
}
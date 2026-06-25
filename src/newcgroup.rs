pub fn set_cgroup(container_id: &str) -> anyhow::Result<()>{
    std::fs::write("/sys/fs/cgroup/cgroup.subtree_control", "+pids +memory +cpu")?;
    let cgroup_path = format!("/sys/fs/cgroup/{}/", container_id);
    std::fs::create_dir_all(&cgroup_path)?;
    std::fs::write(format!("/sys/fs/cgroup/{}/memory.max", container_id), "536870912")?;
    println!("memory limit set to 512MB");
    std::fs::write(format!("/sys/fs/cgroup/{}/pids.max", container_id), "512")?;
    println!("pids limit set to 512");
    Ok(())
}

pub fn add_to_cgroup(container_id: &str, pid: u32) -> anyhow::Result<()> {
    std::fs::write(format!("/sys/fs/cgroup/{}/cgroup.procs", container_id), pid.to_string())?;
    println!("pid {} added to cgroup", pid);
    Ok(())
}
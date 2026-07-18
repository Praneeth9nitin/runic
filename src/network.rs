

pub async fn set_network(container_id: &str) -> anyhow::Result<()>{
    create_bridge().await?;
    establish_connection(container_id).await?;
    connect_bridge_to_veth(container_id).await?;
    setup_nat()?;
    Ok(())
}

pub async fn create_bridge() -> anyhow::Result<()>{
    let check = std::process::Command::new("ip")
        .args(["link", "show", "runic0"])
        .output()?;

    if check.status.success(){
        println!("bridge already exist");
        return Ok(());
    }
    let out = std::process::Command::new("sudo")
        .args(["ip", "link", "add", "name", "runic0", "type", "bridge"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("runic0 bridge created");

    let out = std::process::Command::new("sudo")
        .args(["ip", "addr", "add", "10.0.0.1/24", "dev", "runic0"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }
    
    println!("address set to runic0");

    let out = std::process::Command::new("sudo")
        .args(["ip", "link", "set", "dev", "runic0", "up"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("host network up");

    Ok(())
}

pub async fn establish_connection(container_id: &str) -> anyhow::Result<()>{
    let veth1 = format!("veth0_{}", container_id);
    let veth2 = format!("veth1_{}", container_id);

    let check = std::process::Command::new("ip")
        .args(["link", "show", &veth1])
        .output()?;
    let check2 = std::process::Command::new("ip")
        .args(["link", "show", &veth2])
        .output()?;

    if check.status.success() {
        let out = std::process::Command::new("sudo")
        .args(["ip", "link", "delete", &veth1])
        .output()?;
    }

    if check2.status.success(){
        let out = std::process::Command::new("sudo")
        .args(["ip", "link", "delete", &veth2])
        .output()?;
    }
    let out = std::process::Command::new("sudo")
        .args(["ip", "link", "add", &veth1, "type", "veth", "peer", "name", &veth2])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!("line 50 {}", error));
    }

    println!("creating veth pair: {} <-> {}", veth1, veth2);
    Ok(())
}
pub async fn connect_bridge_to_veth(container_id: &str) -> anyhow::Result<()>{
    let veth1 = format!("veth0_{}", container_id);
    let out = std::process::Command::new("sudo")
        .args(["ip", "link", "set", &veth1, "master", "runic0"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("veth is connected to host");
    let out = std::process::Command::new("sudo")
        .args(["ip", "link", "set", &veth1, "up"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("veth host is up now");
    Ok(())
}


pub async fn add_container(container_id: &str, child_pid: i32) -> anyhow::Result<()>{
    let veth2 = format!("veth1_{}", container_id);
    let out = std::process::Command::new("sudo")
    .args(["nsenter", "--target", &child_pid.to_string(), "--net",
           "ip", "link", "set", "lo", "up"])
    .output()?;
     println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
    println!("status: {}", out.status);

    let out = std::process::Command::new("ip")
        .args(["link", "set", &veth2, "netns", &child_pid.to_string()])
        .output()?;
    println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
    println!("status: {}", out.status);

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("veth is connected to child process");
    Ok(())
}

pub async fn container_network_configuration(container_id: &str, child_pid: i32) -> anyhow::Result<()>{
    let veth2 = format!("veth1_{}", container_id);
    let pid_str = child_pid.to_string();

    std::process::Command::new("sudo")
    .args(["nsenter" ,"--target", &pid_str, "--net", "ip", "addr", "flush", "dev", &veth2])
    .output()?;

    let out = std::process::Command::new("sudo")
        .args(["nsenter", "--target", &pid_str, "--net", "ip", "addr", "add", "10.0.0.2/24", "dev", &veth2])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        println!("{}",error);
        return Err(anyhow::anyhow!(error));
    }

    println!("network set for child for pid {}", child_pid);

    let out = std::process::Command::new("sudo")
        .args(["nsenter", "--target", &pid_str, "--net",
               "ip", "link", "set", &veth2, "up"])
        .output()?;

    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

     let out = std::process::Command::new("sudo")
    .args(["nsenter", "--target", &child_pid.to_string(), "--net",
           "ip", "route", "add", "default", "via", "10.0.0.1"])
    .output()?;
    println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
    println!("status: {}", out.status);

    println!("child ethernet is up for pid {}", child_pid);

    Ok(())
}

pub fn setup_nat() -> anyhow::Result<()> {
    let check: std::process::Output = std::process::Command::new("cat")
        .args(["/proc/sys/net/ipv4/ip_forward"])
        .output()?;

    let value = String::from_utf8_lossy(&check.stdout).trim().to_string();

    if value != "1" {
        let out = std::process::Command::new("sysctl")
            .args(["-w", "net.ipv4.ip_forward=1"])
            .output()?;
        if !out.status.success() {
            let error = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(anyhow::anyhow!(error));
        }
        println!("ip_forward enabled");
    } else {
        println!("ip_forward already enabled");
    }

    let check: std::process::Output = std::process::Command::new("iptables")
        .args(["-t", "nat", "-C", "POSTROUTING", "-s", "10.0.0.0/24", "!", "-o", "runic0", "-j", "MASQUERADE"])
        .output()?;

    if check.status.success(){
        println!("nat already exist");
        return Ok(());
    }
    let out = std::process::Command::new("iptables")
        .args(["-t", "nat", "-A", "POSTROUTING",
               "-s", "10.0.0.0/24",
               "-j", "MASQUERADE"])
        .output()?;
        
    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(anyhow::anyhow!(error));
    }

    println!("NAT done");
    Ok(())
}
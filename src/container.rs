use crate::namespace::set_namespace;
use crate::filesystem::set_filesystem;
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use nix::unistd::{ForkResult, close, execve, fork, pipe, read};
use serde::{Deserialize, Serialize};
use crate::newcgroup::{set_cgroup, add_to_cgroup};
use crate::network::{set_network, container_network_configuration, add_container};
use std::ffi::{CString};
use std::process::Child;


#[derive(Serialize, Deserialize)]
pub enum ContainerState {
    Created,
    Running { pid: u32 },
    Exited { code: i32 },
}

pub struct Container {
    pub id: String,
    pub state: ContainerState,
    child: Option<Child>,
}

impl Container {
    pub fn new(id: &str) -> Self {
        Container {
            id: id.to_string(),
            state: ContainerState::Created,
            child: None,
        }
    }
    pub async fn run(&mut self, program: &str, rootfs_path: String, slave: Option<OwnedFd>, ip: String, hostname:String) -> anyhow::Result<()> {
        let (read_fd, write_fd) = pipe()?;
        let (read_fd_1, write_fd_1) = pipe()?;
        let id = self.id.clone();
        let rootfs = rootfs_path.clone();

       match unsafe{fork()} {
            Ok(ForkResult::Parent { child })=>{
                close(read_fd)?;
                close(write_fd_1)?;
                set_cgroup(&self.id)?;
                set_network(&self.id).await?;
                add_to_cgroup(&self.id, child.as_raw())?;
                close(write_fd)?;
                let mut buffer = [0u8; 32];
                read(read_fd_1, &mut buffer)?;
                add_container(&self.id, child.as_raw()).await.expect("msg");
                container_network_configuration(&self.id, child.as_raw(), ip).await.expect("msg");
                let file_path = format!("/tmp/runic/containers/{}/config.json",&self.id);
                let mut file = File::open(&file_path)?;
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                let mut json_data: serde_json::Value = serde_json::from_str(&content)?;
                json_data["state"] = serde_json::to_value(ContainerState::Running{pid: child.as_raw() as u32 })?;

                let updated = serde_json::to_string_pretty(&json_data)?;
                std::fs::write(&file_path, updated)?;
                self.state = ContainerState::Running { pid: child.as_raw() as u32 };
                
            }
            Ok(ForkResult::Child) => {
                if let Some(ref slave_result) = slave {
                    nix::unistd::setsid()?;
                    let mut stdin_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(0) };
                    let mut stdout_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(1) };
                    let mut stderr_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(2) };
                    nix::unistd::dup2(&slave_result, &mut stdin_fd)?;
                    nix::unistd::dup2(&slave_result, &mut stdout_fd)?;
                    nix::unistd::dup2(&slave_result, &mut stderr_fd)?;
                    std::mem::forget(stdin_fd);
                    std::mem::forget(stdout_fd);
                    std::mem::forget(stderr_fd);
                    println!("hello");
                }
                close(read_fd_1)?;
                close(write_fd)?;
                let mut buffer = [0u8; 32];
                read(read_fd, &mut buffer)?;
                set_namespace(hostname)?;
                set_filesystem(&id, &rootfs)?;
                let path = CString::new(program).unwrap();
                let args = [CString::new("bash").unwrap()];
                let env = vec![
                    CString::new("PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin").unwrap(),
                    CString::new("HOME=/root").unwrap(),
                    CString::new("TERM=xterm").unwrap(),
                    ];
                    
                close(write_fd_1)?;
                execve(&path, &args, &env)?;
            }
            Err(_) => {println!("fork failed");}
        };
        Ok(())
    }
    pub fn wait(&mut self) -> anyhow::Result<i32> {
        if let Some(mut child) = self.child.take() {
            let status = child.wait()?;
            let code = status.code().unwrap_or(-1);
            self.state = ContainerState::Exited { code };
            println!("Container {} exited with code {}", self.id, code);
            
            let veth = format!("veth0_{}",self.id);
            std::process::Command::new("ip")
                .args(["link", "delete", &veth])
                .output()
                .ok();

            let merged = format!("/tmp/runic/{}/merged", self.id);
            std::process::Command::new("umount")
                .args(["-l", &merged])
                .output()
                .ok();

            // std::fs::remove_dir_all(format!("/tmp/runic/{}", self.id)).ok();
            Ok(code)
        } else {
            Ok(-1)
        }
    }
    pub fn status(&self) -> &ContainerState {
        &self.state
    }
}

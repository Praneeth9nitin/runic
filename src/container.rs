use crate::namespace::set_namespace;
use crate::filesystem::set_filesystem;
// use crate::newcgroup::{set_cgroup, add_to_cgroup};
use crate::network::{set_network, container_network_configuration, add_container};
use std::{
    os::unix::process::CommandExt,
    process::{Child, Command},
};

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
    pub fn new(id: String) -> Self {
        Container {
            id: id,
            state: ContainerState::Created,
            child: None,
        }
    }
    pub async fn run(&mut self, program: &str, rootfs_path: String) -> anyhow::Result<()> {
        let id = self.id.clone();
        // set_cgroup(&self.id)?;
        set_network(&self.id).await?;
        let child = unsafe {
            Command::new(program)
            .pre_exec(move || {
                set_namespace()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                set_filesystem(&id, &rootfs_path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                Ok(())
                })
                .spawn()
        }?;
        let pid = child.id();
        add_container(&self.id, pid).await?;

        container_network_configuration(&self.id, pid).await?;

        self.child = Some(child);
        self.state = ContainerState::Running { pid };
        println!("Container {} running with pid {}", self.id, pid);
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

            std::fs::remove_dir_all(format!("/tmp/runic/{}", self.id)).ok();
            Ok(code)
        } else {
            Ok(-1)
        }
    }
    pub fn status(&self) -> &ContainerState {
        &self.state
    }
}

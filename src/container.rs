use std::process::{Child, Command};

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
    pub fn run(&mut self, program: &str) -> anyhow::Result<()> {
        let child = Command::new(program).spawn()?;
        let pid = child.id();

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
            Ok(code)
        } else {
            Ok(-1)
        }
    }
    pub fn status(&self) -> &ContainerState {
        &self.state
    }
}

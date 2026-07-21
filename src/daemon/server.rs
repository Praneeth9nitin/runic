use std::os::fd::{AsRawFd, OwnedFd};
use uuid::Uuid;
use tokio::fs::File;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use crate::container::{
    ContainerState::{Created, Running},
    Container
};
use crate::daemon::store::ContainerConfig;
use crate::proto::{Command, Response};
use std::os::unix::io::BorrowedFd;

pub fn get_file(cont_id:&str)->String{
    format!("/tmp/runic/{}/config.json",cont_id)
}

pub async fn read_file(cont_id:&str)->anyhow::Result<String>{
    let file = File::open(get_file(&cont_id)).await.expect("file not found");
    let mut string_reader = BufReader::new(file);
    let mut content = String::new();
    string_reader.read_to_string(&mut content).await?;
    Ok(content)
}

pub async fn end(pid: String) -> anyhow::Result<()>{
    let veth = format!("veth0_{}", pid);
    std::process::Command::new("ip")
        .args(["link", "delete", &veth])
        .output().ok();
    let merged = format!("/tmp/runic/{}/merged", pid);
    std::process::Command::new("umount")
        .args(["-l", &merged])
        .output().ok();
    std::fs::remove_dir_all(format!("/tmp/runic/{}", pid)).ok();
    Ok(())
}

pub async fn start() -> anyhow::Result<()> {
    let socket_path = "/tmp/runic.sock";

    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        println!("{:?}",stream);
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(mut stream: UnixStream)->anyhow::Result<()>{
    let (mut reader, mut writer) = stream.into_split();
    let mut buffered_reader = BufReader::new(reader);
    let mut line = String::new();

    if buffered_reader.read_line(&mut line).await? > 0 {
        let request: Command = serde_json::from_str(&line)?;

        let response = match request{
            Command::Create { image, tag } =>{
                let rootfs_path = crate::image::pull(&image, &tag).await.unwrap();
                let id = format!("{}",Uuid::new_v4())[..8].to_string();
                let cont_data = ContainerConfig{id:id, image:image, tag:tag, state:Created, rootfs:rootfs_path};
                let file_path = format!("/tmp/runic/{}/",cont_data.id);
                std::fs::create_dir(&file_path)?;
                let write_file = File::create(format!("{}/config.json",&file_path)).await?;
                let mut writer = BufWriter::new(write_file);
                let json_string = serde_json::to_string_pretty(&cont_data)?;
                writer.write_all(json_string.as_bytes()).await?;
                writer.flush().await?;
                let send_data = format!("container id {}", cont_data.id);
                let res = Response::Ok { message:send_data};
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
            }
            Command::Start { cont_id, program } =>{
                let content = read_file(&cont_id).await.unwrap();
                let data:ContainerConfig = serde_json::from_str(&content)?;
                let mut c = crate::container::Container::new(data.id);
            }
            Command::Exec { cont_id, command, interactive } =>{
                let content = read_file(&cont_id).await.unwrap();
                let data:ContainerConfig = serde_json::from_str(&content)?;
                println!("{}", &content);
                let pid = match data.state {
                    Running { pid } => pid,
                    _ => return Err(anyhow::anyhow!("container {} is not running", cont_id)),
                };
                let args: Vec<&str> = command.split_whitespace().collect();
                let output = std::process::Command::new("nsenter")
                    .args(["--target",&pid.to_string(), "--all"])
                    .args(&args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .output()?;
                println!("running nsenter for pid: {}", pid);
                println!("command: {}", command);
                println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

                let res = Response::ExecOutput { stdout: String::from_utf8_lossy(&output.stdout).to_string(), stderr: String::from_utf8_lossy(&output.stderr).to_string() };
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
                nix::sys::wait::waitpid( nix::unistd::Pid::from_raw(pid as i32), None)?;
                
            }
            Command::Run{cont_id , program} =>{
                let content = read_file(&cont_id).await.unwrap();
                let data:ContainerConfig = serde_json::from_str(&content)?;
                let pty = nix::pty::openpty(None, None)?;
                let mut c = crate::container::Container::new(data.id);
                Container::run(&mut c, &program, data.rootfs, Some(pty.slave)).await?;
                let master_fd = pty.master.as_raw_fd();
                std::mem::forget(pty.master);
                let task1 = tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    loop {
                        let n = buffered_reader.read(&mut buf).await?;
                        if n == 0 { break; }
                        nix::unistd::write(unsafe { BorrowedFd::borrow_raw(master_fd) }, &buf[..n])?;
                    }
                    Ok::<(), anyhow::Error>(())
                });

                let task2 = tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    loop {
                        let borrowed = unsafe { BorrowedFd::borrow_raw(master_fd) };
                        match nix::unistd::read(borrowed, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => { writer.write_all(&buf[..n]).await?; }
                            Err(_) => break,
                        }
                    }
                    Ok::<(), anyhow::Error>(())
                });

                tokio::select! {
                    _ = task1 => {}
                    _ = task2 => {}
                }
                let pid = match data.state {
                    Running { pid } => pid,
                    _ => return Err(anyhow::anyhow!("container not running")),
                };
                nix::sys::wait::waitpid( nix::unistd::Pid::from_raw(pid as i32), None)?;
            },
            Command::Stop{container_id}=>{
                println!("work in progress")
            },
            _ => {
                println!("work in progress")
            }

        };
    }
    Ok(())
}
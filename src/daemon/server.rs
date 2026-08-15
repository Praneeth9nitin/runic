use std::os::fd::{AsRawFd};
use uuid::Uuid;
use tokio::fs::File;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use crate::container::{
    ContainerState::{Created, Running},
    Container
};
use crate::daemon::store::ContainerConfig;
use crate::proto::{Command, ContainerInfo, ImageInfo, Response};
use std::os::unix::io::BorrowedFd;
use walkdir::WalkDir;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, Pid};

pub fn get_file(cont_id:&str)->String{
    format!("/tmp/runic/containers/{}/config.json",cont_id)
}

pub async fn read_file(cont_id:&str)->anyhow::Result<String>{
    let file = File::open(get_file(&cont_id)).await.expect("file not found");
    let mut string_reader = BufReader::new(file);
    let mut content = String::new();
    string_reader.read_to_string(&mut content).await?;
    Ok(content)
}

pub async fn end(cont_id: &str) -> anyhow::Result<()>{
    let veth = format!("veth0_{}", cont_id);
    std::process::Command::new("ip")
        .args(["link", "delete", &veth])
        .output().ok();
    let merged = format!("/tmp/runic/containers/{}/merged", cont_id);
    std::process::Command::new("umount")
        .args(["-l", &merged])
        .output().ok();
    std::fs::remove_dir_all(format!("/tmp/runic/containers/{}", cont_id)).ok();
    Ok(())
}

pub async fn kill_process_by_pid(pid_num: u32) {
    let sys = System::new_with_specifics(
        RefreshKind::everything().with_processes(ProcessRefreshKind::everything())
    );

    let target_pid = Pid::from(pid_num as usize);

    if let Some(process) = sys.process(target_pid) {
        println!("Found process: {}. Stopping it...", process.name().to_string_lossy());
        process.kill();
    } else {
        println!("No running process found with PID: {}", pid_num);
    }
}

pub fn allocate_ip()-> anyhow::Result<String>{
    let counter_path = "/tmp/runic/ip_counter";

    let counter: u8 = std::fs::read_to_string(counter_path)
        .unwrap_or("2".to_string())
        .trim()
        .parse()
        .unwrap_or(2);
    std::fs::write(counter_path, (counter + 1).to_string())?;

    Ok(format!("10.0.0.{}", counter))
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

pub fn get_dir_size(path: &str) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

async fn handle_connection(stream: UnixStream)->anyhow::Result<()>{
    let (reader, mut writer) = stream.into_split();
    let mut buffered_reader = BufReader::new(reader);
    let mut line = String::new();

    if buffered_reader.read_line(&mut line).await? > 0 {
        let request: Command = serde_json::from_str(&line)?;

        let response = match request{
            Command::Create { image, tag } =>{
                let tag1 = tag.clone();
                let rootfs_path = crate::image::pull(&image, &tag).await.unwrap();
                let id = format!("{}",Uuid::new_v4())[..8].to_string();
                let image = ImageInfo{name:image, tag:tag, size:get_dir_size(&rootfs_path)};
                let cont_data = ContainerConfig{id:id, image:image, tag:tag1, state:Created, rootfs:rootfs_path, ip:allocate_ip().unwrap()};
                let file_path = format!("/tmp/runic/containers/{}/",cont_data.id);
                std::fs::create_dir_all(&file_path).expect("msg");
                let write_file = File::create(format!("{}/config.json",&file_path)).await?;
                let mut buff_writer = BufWriter::new(write_file);
                let json_string = serde_json::to_string_pretty(&cont_data)?;
                buff_writer.write_all(json_string.as_bytes()).await?;
                buff_writer.flush().await?;
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
                let mut c = crate::container::Container::new(&data.id);
                Container::run(&mut c, &program, data.rootfs, None, data.ip, data.id).await?;
                let pid = match data.state {
                    Running { pid } => pid,
                    _ => return Err(anyhow::anyhow!("container not running")),
                };
                nix::sys::wait::waitpid( nix::unistd::Pid::from_raw(pid as i32), None)?;
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

                let res = Response::ExecOutput { stdout: String::from_utf8_lossy(&output.stdout).to_string(), stderr: String::from_utf8_lossy(&output.stderr).to_string() };
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
            }
            Command::Run{cont_id , program} =>{
                let content = read_file(&cont_id).await.unwrap();
                let data:ContainerConfig = serde_json::from_str(&content)?;
                let pty = nix::pty::openpty(None, None)?;
                let mut c = crate::container::Container::new(&data.id);
                tokio::spawn(async move{
                    Container::run(&mut c, &program, data.rootfs, Some(pty.slave), data.ip, data.id).await
                    .unwrap_or_else(|e| eprintln!("container error: {}", e));
                });
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let master_fd = pty.master.as_raw_fd();
                std::mem::forget(pty.master);
                eprintln!("runicd: starting PTY forwarding");
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
            Command::Ps=>{
                let mut containers: Vec<ContainerInfo> = Vec::new();
                let entries : Vec<_> = WalkDir::new("/tmp/runic/containers/")
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .map(|e| e.into_path())
                    .collect();
                for path in entries{
                    let file_name = format!("{}/config.json", path.to_string_lossy());
                    let file = File::open(file_name).await.expect("file not found");
                    let mut string_reader = BufReader::new(file);
                    let mut content = String::new();
                    string_reader.read_to_string(&mut content).await?;
                    let data:ContainerConfig = serde_json::from_str(&content)?;
                    let mut some_pid = 0;
                    let status = match data.state{
                        Running { pid } => {
                            some_pid = pid;
                            "running"
                        },
                        Created =>{
                            "created"
                        },
                        _ => return Err(anyhow::anyhow!("no container")),
                    };
                    let entry = ContainerInfo{id:data.id, image:data.image.name, status:status.to_string(), pid:some_pid};
                    containers.push(entry);
                }
                let response = Response::ContainerList { containers };
                let mut json = serde_json::to_string(&response)?;
                json.push('\n');
                writer.write_all(json.as_bytes()).await?;
                writer.flush().await?;
            },
            Command::Stop { container_id } =>{
                let file_path = get_file(&container_id);
                let mut file = File::open(&file_path).await?;
                let mut content = String::new();
                file.read_to_string(&mut content).await?;
                let data:ContainerConfig = serde_json::from_str(&content)?;
                let pid = match data.state{
                    Running{pid}=>{
                        pid
                    }
                    _=> return Err(anyhow::anyhow!("container not running")),
                };
                let mut json_data: serde_json::Value = serde_json::from_str(&content)?;
                json_data["state"] = serde_json::to_value(Created)?;

                let updated = serde_json::to_string_pretty(&json_data)?;
                std::fs::write(&file_path, updated)?;
                kill_process_by_pid(pid).await;
                let msg = format!("process {} deleted",pid);
                let res = Response::Ok { message: msg };
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
            }
            Command::Rm { container_id } =>{
                end(&container_id).await?;
                let msg = format!("container {} removed",container_id);
                let res = Response::Ok { message: msg };
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
            }
            Command::Images =>{
                let mut images: Vec<ImageInfo> = Vec::new();
                let image_list : Vec<_> = WalkDir::new("/tmp/runic/rootfs/")
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .map(|e| e.into_path())
                    .collect();
                for image in image_list{
                    let size = get_dir_size(&image.to_string_lossy());
                    let tag: Vec<_> = WalkDir::new(&image)
                        .min_depth(1)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .map(|e| e.into_path())
                        .collect();
                    let image_path_str = image.display().to_string();
                    let name_parts: Vec<&str> = image_path_str.split("/").collect();
                    let tag_str = tag[0].display().to_string();
                    let tag_slice: Vec<&str> = tag_str.split("/").collect();
                    let value = ImageInfo{name: name_parts[4].to_string()[8..].to_string(), tag:tag_slice[5].to_string(), size};
                    images.push(value);
                }
                let res = Response::ImageList { images };
                let mut json_data = serde_json::to_string(&res)?;
                json_data.push('\n');
                writer.write_all(json_data.as_bytes()).await?;
                writer.flush().await?;
            }
            _ => {
                println!("work in progress")
            }

        };
    }
    Ok(())
}
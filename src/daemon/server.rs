use uuid::Uuid;
use tokio::fs::File;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use crate::container::{
    ContainerState::Created,
    Container
};
use crate::daemon::store::ContainerConfig;
use crate::proto::{Command, Response};

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
    let (reader,_) = stream.split();
    let mut buffered_reader = BufReader::new(reader);
    let mut line = String::new();

    if buffered_reader.read_line(&mut line).await? > 0 {
        let request: Command = serde_json::from_str(&line)?;

        let response = match request{
            Command::Create { image, tag } =>{
                let rootfs_path = crate::image::pull(&image, &tag).await.unwrap();
                let id = format!("{}",Uuid::new_v4())[..8].to_string();
                let cont_data = ContainerConfig::new(id, image, tag, Created, rootfs_path);
                let file_path = format!("/tmp/runic/{}",cont_data.id);
                std::fs::create_dir(&file_path)?;
                let write_file = File::create(format!("{}/config.json",&file_path)).await?;
                let mut writer = BufWriter::new(write_file);
                let json_string = serde_json::to_string_pretty(&cont_data)?;
                writer.write_all(json_string.as_bytes()).await?;
                writer.flush().await?;
            }
            Command::Start { cont_id, program } =>{
                let file = File::open(format!("/tmp/runic/{}/config.json", cont_id)).await.expect("file not found");
                let mut reader = BufReader::new(file);
                let mut content = String::new();
                println!("im i start");
                reader.read_to_string(&mut content).await?;
                println!("{}", content);
                let data:ContainerConfig = serde_json::from_str(&content)?;
                let mut c = crate::container::Container::new(data.id);
                Container::run(&mut c, &program, data.rootfs).await?;
                
            }
            Command::Run{image  , tag, program} =>{
                let rootfs_path = crate::image::pull(&image, &tag).await.unwrap();
                let mut c = crate::container::Container::new(format!("{}",Uuid::new_v4())[..8].to_string());
                if let Err(e) = c.run(&program, rootfs_path).await{
                    eprintln!("error: {:#}", e);
                }
                if let Err(e) = c.wait() {
                    eprintln!("wait error: {:#}", e);
                }
            },
            // Command::Stop{container_id}=>{
            //     println!("work in progress")
            // },
            _ => {
                println!("work in progress")
            }

        };
    }
    Ok(())
}
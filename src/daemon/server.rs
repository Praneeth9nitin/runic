use uuid::Uuid;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    let (reader, mut writer) = stream.split();
    let mut buffered_reader = BufReader::new(reader);
    let mut line = String::new();

    if buffered_reader.read_line(&mut line).await? > 0 {
        let request: Command = serde_json::from_str(&line)?;

        let response = match request{
            Command::Run{image  , tag, program} =>{
                let rootfs_path = crate::image::pull(&image, &tag).await.unwrap();
                let mut c = crate::container::Container::new(format!("runic_{}",Uuid::new_v4()).to_string());
                if let Err(e) = c.run(&program, rootfs_path).await{
                    eprintln!("error: {:#}", e);
                }
                if let Err(e) = c.wait() {
                    eprintln!("wait error: {:#}", e);
                }
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
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt};
use clap:: {Parser, Subcommand};
use runic::proto::{Command, Response};

#[derive(Debug, Parser)]
#[command(name = "runic")]

struct Cli{
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]

enum Commands{
    Create{
        image : String,
    },
    Start{
        cont_id : String,
        program : String
    },
    Run{
        image : String,
        program : String
    },
    Ps,
    Stop {
        id : String
    },
    Logs {
        id : String
    },
    Images,
    Rm {
        id : String
    }
}
#[tokio::main]
async fn main()-> anyhow::Result<()>{
    let args = Cli::parse();

    let socket_path = format!("/tmp/runic.sock");
    let mut stream = UnixStream::connect(socket_path).await?;
    
    match args.command {
        Commands::Create { image } => {
           let parts : Vec<&str> = image.trim().split(":").collect();
           let image_name = format!("library/{}", parts[0]);
           let tag = parts.get(1).unwrap_or(&"latest").to_string();
           let req = Command::Create { image:image_name, tag };
           let mut json_payload = serde_json::to_string(&req)?;
           json_payload.push('\n');

           let (_, mut writer) = stream.split();
           writer.write_all(json_payload.as_bytes()).await?;
           writer.flush().await?;
        }
        Commands::Start { cont_id, program } =>{
            let (_ , mut writer) = stream.split();
            let req = Command::Start { cont_id, program };
            let mut json_payload = serde_json::to_string(&req)?;
            json_payload.push('\n');
            writer.write_all(json_payload.as_bytes()).await?;
            writer.flush().await?;
        }
        Commands::Run{image, program} =>{
           let parts : Vec<&str> = image.trim().split(":").collect();
           let image_name = format!("library/{}", parts[0]);
           let tag = parts.get(1).unwrap_or(&"latest").to_string();
           let req = Command::Run {
            image:image_name,
            tag:tag,
            program:program,
           };

           let mut json_payload = serde_json::to_string(&req)?;
           json_payload.push('\n');

           let (_ , mut writer) = stream.split();
           writer.write_all(json_payload.as_bytes()).await?;
           writer.flush().await?;
        },
        _ => {println!("work in progress");}
    };
    Ok(())
}
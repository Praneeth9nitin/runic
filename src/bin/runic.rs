use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use clap:: {Parser, Subcommand};
use runic::proto::{Command, Response};

#[derive(Debug, Parser)]
#[command(name = "runic")]

struct Cli{
    #[command[subcommand]]
    command: Commands,
}

#[derive(Debug, Subcommand)]

enum Commands{
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
async fn main(){
    let args = Cli::parse();

    let socket_path = format!("/tmp/runic.sock");
    let mut stream = UnixStream::connect(socket_path);
    
    match args.command {
        Commands::Run{image, program} =>{
            let image_vec: &Vec<&str>  = &image.trim().split(":").collect();
            let image_name = format!("library/{}",image_vec[0]);
            let req = Command::Run{image:image_name, tag: image_vec[1].to_string(), program};
            let mut json_payload = serde_json::to_string(&req)?;
            json_payload.push('\n');

            let (reader, mut writer) = stream.split();
            writer.write_all(json_payload.as_bytes())?;
            writer.flush()?;
        },
        _ => {println!("work in progress");}
    }
}
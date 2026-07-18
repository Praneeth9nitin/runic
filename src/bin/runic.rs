use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    Exec{
        cont_id : String,
        #[arg(trailing_var_arg = true, num_args = 1..)]
        command: Vec<String>,
        #[arg(short = 'i', long)]
        interactive : bool
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
    let (mut reader , mut writer) = stream.split();
    
    match args.command {
        Commands::Create { image } => {
           let parts : Vec<&str> = image.trim().split(":").collect();
           let image_name = format!("library/{}", parts[0]);
           let tag = parts.get(1).unwrap_or(&"latest").to_string();
           let req = Command::Create { image:image_name, tag };
           let mut json_payload = serde_json::to_string(&req)?;
           json_payload.push('\n');
           writer.write_all(json_payload.as_bytes()).await?;
           writer.flush().await?;
        }
        Commands::Start { cont_id, program } =>{
            
            let req = Command::Start { cont_id, program };
            let mut json_payload = serde_json::to_string(&req)?;
            json_payload.push('\n');
            writer.write_all(json_payload.as_bytes()).await?;
            writer.flush().await?;
        }
        Commands::Exec {cont_id, command, interactive} => {
            let cmd = command.join(" ");
            let req = Command::Exec{cont_id, command:cmd, interactive};
            let mut json_payload = serde_json::to_string(&req)?;
            json_payload.push('\n');
            writer.write_all(json_payload.as_bytes()).await?;
            writer.flush().await?;
            let mut line = String::new();
            reader.read_to_string(&mut line).await?;
            let response: Response = serde_json::from_str(&line)?;
            match response {
                Response::ExecOutput { stdout, stderr } => {
                    print!("{}", stdout);
                    if !stderr.is_empty() {
                        eprint!("{}", stderr);
                    }
                }
                Response::Error { message } => {
                    eprintln!("error: {}", message);
                }
                _ => {}
            }
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

           writer.write_all(json_payload.as_bytes()).await?;
           writer.flush().await?;
        },
        _ => {println!("work in progress");}
    };
    Ok(())
}
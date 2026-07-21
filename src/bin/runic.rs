use std::os::fd::AsRawFd;
use std::os::unix::io::BorrowedFd;
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
        cont_id : String,
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
    let (mut reader , mut writer) = stream.into_split();
    
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
           let mut line = String::new();
            reader.read_to_string(&mut line).await?;
            let response: Response = serde_json::from_str(&line)?;
            match response {
                Response::Ok { message} => {
                    print!("{}", message);
                }
                _ => {}
            }
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
        Commands::Run{cont_id, program} =>{
           let req = Command::Run {
            cont_id,
            program
           };

           let mut json_payload = serde_json::to_string(&req)?;
           json_payload.push('\n');

           writer.write_all(json_payload.as_bytes()).await?;
           writer.flush().await?;
           let stdin_fd = std::io::stdin().as_raw_fd();
           let borrowed = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
           let original = nix::sys::termios::tcgetattr(borrowed)?;
           let mut raw = original.clone();
           nix::sys::termios::cfmakeraw(&mut raw);
           nix::sys::termios::tcsetattr(borrowed, nix::sys::termios::SetArg::TCSANOW, &raw)?;

           let task1 : tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move{
            let mut stdin = tokio::io::stdin();
            let mut buf = [0u8; 1024];
            loop{
                let n = stdin.read(&mut buf).await?;
                if n==0 {break;}
                writer.write_all(&buf[..n]).await?;
                writer.flush().await?;
            }
            Ok(())
           });

           let task2 : tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move{
            let mut stdout = tokio::io::stdout();
            let mut buf = [0u8; 1024];
            loop{
                let n = reader.read(&mut buf).await?;
                if n == 0 { break; }
                stdout.write_all(&buf[..n]).await?;
                stdout.flush().await?;
            }
            Ok(())
           });

           tokio::select! {
            _ = task1 =>{}
            _ = task2 =>{}
           }
           nix::sys::termios::tcsetattr(borrowed, nix::sys::termios::SetArg::TCSANOW, &original)?;
        },
        _ => {println!("work in progress");}
    };
    Ok(())
}
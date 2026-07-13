#[tokio::main]
async fn main() -> anyhow::Result<()>{
    println!("runicd is starting");
    runic::daemon::server::start().await?;
    Ok(())
}
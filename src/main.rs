mod container;
mod filesystem;
mod namespace;
mod newcgroup;
mod image;
use std::env;
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let image_to_pull: &Vec<&str> = &args[1].trim().split(':').collect();
    let act_img = format!("library/{}",image_to_pull[0]);
    let rootfs_path = image::pull(&act_img, &image_to_pull[1]).await.unwrap();
    let mut c = container::Container::new("cont1".to_string());
    if let Err(e) = c.run(&args[2], rootfs_path) {
        eprintln!("error: {:#}", e);  // #  shows full error chain
        std::process::exit(1);
    }
    
    if let Err(e) = c.wait() {
        eprintln!("wait error: {:#}", e);
    }
}

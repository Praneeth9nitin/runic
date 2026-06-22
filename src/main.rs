mod container;
mod filesystem;
mod namespace;
use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut c = container::Container::new("cont1".to_string());
    if let Err(e) = c.run(&args[1]) {
        eprintln!("error: {:#}", e);  // #  shows full error chain
        std::process::exit(1);
    }
    
    if let Err(e) = c.wait() {
        eprintln!("wait error: {:#}", e);
    }
}

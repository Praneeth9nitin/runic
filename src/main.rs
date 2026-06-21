mod container;
mod namespace;
use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut c = container::Container::new("cont1".to_string());
    c.run(&args[1]).unwrap();
    let code = c.wait().unwrap();
    println!("exited with code {code}");
}

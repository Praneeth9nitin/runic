mod container;
mod namespace;
fn main() {
    let mut c = container::Container::new("cont1".to_string());
    c.run("/bin/bash").unwrap();
    let code = c.wait().unwrap();
    println!("exited with code {code}");
}

mod container;
fn main() {
    let mut cont1 = container::Container::new("cont1".to_string());
    println!("container is create with id {}", cont1.id);
    cont1.run("ls").unwrap();
    match cont1.status() {
        container::ContainerState::Running { pid } => {
            println!("container is running with pid {}", pid);
        }
        _ => {}
    }
    cont1.wait().unwrap();
    match cont1.status() {
        container::ContainerState::Exited { code } => {
            println!("container is exited with code {}", code);
        }
        _ => {}
    }
}

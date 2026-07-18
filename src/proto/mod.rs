use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Command {
    Create { image: String, tag: String},
    Start { cont_id: String, program: String},
    Exec { cont_id: String, command: String, interactive: bool},
    Run { image: String, tag: String, program: String },
    Stop { container_id: String },
    Ps,
    Logs { container_id: String },
    Images,
    Rm { container_id: String },
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Ok { message: String },
    ContainerList { containers: Vec<ContainerInfo> },
    ImageList { images: Vec<ImageInfo> },
    Error { message: String },
    ExecOutput{
        stdout:String,
        stderr:String
    }
}

#[derive(Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub image: String,
    pub status: String,
    pub pid: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    pub tag: String,
    pub size: u64,
}
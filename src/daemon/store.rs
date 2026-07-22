use crate::{container::{Container, ContainerState}, proto::ImageInfo};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ContainerConfig {
    pub id: String,
    pub image: ImageInfo,
    pub tag: String,
    pub state:ContainerState,
    pub rootfs: String,
    pub ip: String
}
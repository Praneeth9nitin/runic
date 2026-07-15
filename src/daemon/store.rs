use crate::container::{ContainerState, Container};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ContainerConfig {
    pub id: String,
    pub image: String,
    pub tag: String,
    pub state:ContainerState,
    pub rootfs: String
}

impl ContainerConfig {
    pub fn new(id: String, image: String, tag: String, state: ContainerState, rootfs: String) -> Self{
        ContainerConfig{
            id,
            image,
            tag,
            state,
            rootfs
        }
    }
}
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
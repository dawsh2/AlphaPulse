//! UDP transport implementation

use crate::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub struct UdpTransport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConfig {
    pub bind_address: SocketAddr,
}

impl UdpTransport {
    pub fn new(_config: UdpConfig) -> Result<Self> {
        Ok(Self)
    }
}

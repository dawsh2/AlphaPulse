//! TCP transport implementation

use crate::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub struct TcpTransport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfig {
    pub bind_address: SocketAddr,
}

pub struct TcpConnection;

impl TcpTransport {
    pub fn new(_config: TcpConfig) -> Result<Self> {
        Ok(Self)
    }
}
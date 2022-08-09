use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;


#[async_trait]
pub trait ZQueueHander: Send + Sync + 'static {
    
    async fn send(&self, msg: (SocketAddr, Bytes))-> Result<()>;

    async fn recv(&self) -> Result<(SocketAddr, Bytes)>;

    fn close(&self) -> bool;
    
}
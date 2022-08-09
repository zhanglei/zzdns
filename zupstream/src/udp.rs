use std::time::Duration;
use std::net::SocketAddr;
use anyhow::Result;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use crate::base::QHandler;


#[derive(Clone)]
pub struct UdpUpstream
{
    server_addr: SocketAddr,
}

impl UdpUpstream {
    pub async fn build(server_addr: SocketAddr) -> Result<Box<dyn QHandler>>{
        Ok(Box::new(Self {
            server_addr
        }))
    }
}

#[async_trait]
impl QHandler for UdpUpstream {
    
    async fn query(&self, qmsg: Bytes, sender: Sender<Bytes>) -> Result<()> {
        let udp_local_addr = "0.0.0.0:0".parse::<SocketAddr>().unwrap();
        let udp_socket = UdpSocket::bind(udp_local_addr).await?;
        let mut buf = BytesMut::with_capacity(1024);
        buf.resize(1024, 0);
        udp_socket.send_to(&qmsg, self.server_addr).await?;
        let (len, _addr) = udp_socket.recv_from(&mut buf).await?;
        buf.resize(len, 0);
        tokio::select! {
            _ = sender.closed() => {}
            _ = sender.send_timeout(buf.freeze(), Duration::from_secs(1)) => {}
        }
        Ok(())
    }

}
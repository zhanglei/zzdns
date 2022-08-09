use anyhow::{anyhow, Result};
use std::net::SocketAddr;
use async_trait::async_trait;
use bytes::Bytes;
use crate::base::ZQueueHander;
use async_channel::{bounded, Sender, Receiver};

#[derive(Debug, Clone)]
pub struct ZResponseQueue
{
    sender: Sender<(SocketAddr, Bytes)>,
    receiver: Receiver<(SocketAddr, Bytes)>,
}

impl ZResponseQueue {

    pub fn new(cap: usize) -> Self {

        let (s, r) = bounded::<(SocketAddr, Bytes)>(cap);
        
        Self { 
            sender: s, 
            receiver: r
        }
    }
}

#[async_trait]
impl ZQueueHander for ZResponseQueue {
    
    async fn send(&self, msg: (SocketAddr, Bytes)) -> Result<()> {
        match self.sender.send(msg).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow!("Send error: {}", err)),
        }
    }

    async fn recv(&self) -> Result<(SocketAddr, Bytes)> {
        match self.receiver.recv().await {
            Ok(msg) => Ok(msg),
            Err(err) => Err(anyhow!("Recv error: {}", err)),
        }
    }

    fn close(&self) -> bool {
        self.receiver.close()
    }
}

#[tokio::test]
    async fn test_zrequest_queue() {
        let qsize = 1024;
        let q = ZResponseQueue::new(qsize);
        let src = "127.0.0.1:10500".parse::<SocketAddr>().unwrap();
        let msg = bytes::Bytes::from_static(b"hello");
        let _ = q.send((src, msg)).await;
        let (recv_src, recv_msg) = q.recv().await.unwrap();
        assert_eq!(src, recv_src);
        assert_eq!(recv_msg, recv_msg);
}
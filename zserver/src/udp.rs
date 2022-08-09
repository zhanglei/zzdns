use std::{sync::Arc, net::SocketAddr};
use anyhow::{Result};
use async_trait::async_trait;
use bytes::BytesMut;
use tokio::{net::UdpSocket, signal};
use crate::base::ZServer;
use zqueue::{ZRequestQueue, ZResponseQueue, ZQueueHander};
use super::ServerConf;

#[derive(Debug, Clone)]
pub struct UdpZserver {
    socket: Arc<UdpSocket>,
    req_q: Arc<ZRequestQueue>,
    res_q: Arc<ZResponseQueue>,
}

impl UdpZserver {

    async fn serve(&self) -> Result<()> {
        info!("zserver running, listening on ({:?})...", self.socket.local_addr().unwrap());

        loop {
            let mut buf = BytesMut::with_capacity(1024);
            buf.resize(1024, 0);

            tokio::select! {
                // Read socket data and send it to the request queue.
                res = self.socket.recv_from(&mut buf) => {
                    let (len, src) = match res {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Fail to read socket data, error:{}", e);
                            continue;
                        }
                    };
                    buf.resize(len, 0);
                    match self.req_q.send((src, buf.freeze())).await {
                        Ok(_) => {},
                        Err(e) => {error!("Failed to push request to queue, error:{:?}", e)}
                    }
                }
                
                res = self.res_q.recv() => {
                    match res {
                        Ok((src, msg)) => {
                            match self.socket.send_to(&msg, src).await{
                                Ok(_) => continue,
                                Err(e) => error!("Failed to send data to client({:?}), error:{:?}", src, e),
                            }
                        },
                        Err(e) => {
                            error!("Failed to read response queue, error:{:?}", e);
                        },
                    }
                }
                // Read the reply queue data and send the data to the client
                // Ok((src, msg)) = self.res_q.recv() => {
                //     match self.socket.send_to(&msg, src).await{
                //         Ok(_) => continue,
                //         Err(e) => error!("Failed to send data to client({:?}), error:{:?}", src, e),
                //     }
                // }
                _ = signal::ctrl_c() => {
                    return self.stop().await;
                }
            }
        }
    }

}

#[async_trait]
impl ZServer for UdpZserver {

    async fn build(req_q: Arc<ZRequestQueue>, res_q: Arc<ZResponseQueue>, conf: ServerConf) -> Result<Self> {
        let socket_addr = format!("0.0.0.0:{}", conf.port).parse::<SocketAddr>()?;
        let socket = Arc::new(UdpSocket::bind(socket_addr).await?);
        Ok(Self { socket, req_q: req_q.clone(), res_q: res_q.clone() })
    }

    async fn start(&self) -> Result<()> {
        self.serve().await
    }

    async fn stop(&self) -> Result<()> {
        info!("Service is down...");
        Ok(())
    }
    
}

#[tokio::test]
async fn test_server() {
    use zconfig::CONFIG;
    let req_q = Arc::new(ZRequestQueue::new(2048));
    let res_q = Arc::new(ZResponseQueue::new(2048));
    let server = UdpZserver::build(req_q, res_q, CONFIG.server.clone()).await.unwrap();
    server.start().await.unwrap();
}

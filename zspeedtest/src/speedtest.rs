
use std::{net::{SocketAddr, Ipv4Addr}, time::Duration, sync::Arc};
use tokio::{net::TcpStream, time::sleep};
use anyhow::{anyhow, Result};
use tokio::sync::{mpsc};
use tokio::sync::mpsc::Sender;
use tokio::sync::Barrier;

pub struct ZSpeedTest;

impl ZSpeedTest {
    
    pub async fn query(ip_list: Vec<Ipv4Addr>) -> Result<Ipv4Addr> {
        if ip_list.is_empty() {
            return Err(anyhow!("empty ip list."));
        }
        if ip_list.len() == 1 {
            return Ok(ip_list[0]);
        }
        let res_ip = ip_list[0];
        
        let (sender, mut receiver) = mpsc::channel::<Ipv4Addr>(1);
        let sender = Arc::new(sender);
        let barrier = Arc::new(Barrier::new(ip_list.len()));
        for ip in ip_list {
            let sender = sender.clone();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                let _ = Self::connect(ip, sender, barrier).await;
            });
        }
        tokio::select! {
            res = receiver.recv() => {
                if let Some(res_ip) = res {
                    receiver.close();
                    return Ok(res_ip);
                }
            },
            _ = sleep(Duration::from_secs(2)) => {
              return Ok(res_ip);
            }
        }
        Ok(res_ip)
    }

    async fn connect(addr: Ipv4Addr, sender: Arc<Sender<Ipv4Addr>>, barrier: Arc<Barrier>) -> Result<()> {
        let sock_addr = format!("{:?}:443", addr).parse::<SocketAddr>().unwrap();
        barrier.wait().await;
        let _ = TcpStream::connect(sock_addr).await?;
        tokio::select! {
            _ = sender.closed() => {}
            _ = sender.send_timeout(addr, Duration::from_secs(2)) => {}
        }
        Ok(())
    }
}
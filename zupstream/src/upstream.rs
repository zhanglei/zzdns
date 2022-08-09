use std::{net::SocketAddr, time::Duration};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use domain::base::{Message, MessageBuilder, iana::Rcode};
use tokio::{sync::mpsc, time::sleep};
use crate::{base::QHandler, udp::{UdpUpstream}, https::HttpsUpstream};
use zconfig::Upstream as UpstreamConf;

#[derive(Clone)]
pub struct ZUpstream {
    upstreams: Vec<Box<dyn QHandler>>
}


impl ZUpstream {

    pub async fn build(upconf_list: Vec<UpstreamConf>) -> Result<Self> {
        
        
        let mut upstreams:Vec<Box<dyn QHandler>> = Vec::new();
        for conf in upconf_list.iter() {
            if let Ok(res) = conf.get_host().parse::<SocketAddr>() {
                let upstream = UdpUpstream::build(res).await?;
                upstreams.push(upstream);
            }else {
                let upstream = HttpsUpstream::build(conf.get_host()).await?;
                upstreams.push(upstream);
            }
        }
        Ok(Self { upstreams })
    }
    
    pub async fn query(&self, qmsg: Bytes) -> Result<Bytes> {
        
        let (sender, mut receiver) = mpsc::channel::<Bytes>(1);
        
        for upstream in self.upstreams.iter() {
            let qmsg = qmsg.clone();
            let sender = sender.clone();
            let upstream = upstream.clone();
            tokio::spawn(async move {
                let _ = upstream.query(qmsg, sender).await;
            });
        }
        let mut res = BytesMut::with_capacity(1024);
        
        tokio::select! {
            buf = receiver.recv() => {
                if let Some(buf) = buf {
                    receiver.close();
                    res.resize(buf.len(), 0);
                    res.copy_from_slice(&buf);
                }else {
                    let buf = MessageBuilder::from_target(BytesMut::with_capacity(1024))?
                    .start_answer(&Message::from_octets(qmsg)?, Rcode::ServFail)?
                    .into_message().into_octets();
                    res.resize(buf.len(), 0);
                    res.copy_from_slice(&buf); 
                }
            },
            _ = sleep(Duration::from_secs(1)) => {
                let buf = MessageBuilder::from_target(BytesMut::with_capacity(1024))?
                .start_answer(&Message::from_octets(qmsg)?, Rcode::ServFail)?
                .into_message().into_octets();
                res.resize(buf.len(), 0);
                res.copy_from_slice(&buf); 
            }
        }
        Ok(res.freeze())
    }

    pub async fn query_all(&self, qmsg: &Bytes) -> Result<Vec<Bytes>> {
        let max_size = self.upstreams.len();
        let (sender, mut receiver) = mpsc::channel::<Bytes>(max_size);
        let mut list = Vec::new();
        let mut handlers = Vec::new();
        for upstream in self.upstreams.iter() {
            let qmsg = qmsg.clone();
            let sender = sender.clone();
            let upstream = upstream.clone();
            let handler = tokio::spawn(async move {
                let _ = upstream.query(qmsg, sender).await;
            });
            handlers.push(handler);
        }
        loop {
            tokio::select! {
                res = receiver.recv() => {
                    if let Some(bytes) = res {
                        list.push(bytes);
                    }
                },
                _ = sleep(Duration::from_secs(1)) => {
                    break;
                }
            }
        }
        Ok(list)
    }
}




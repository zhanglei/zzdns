use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use tokio::{sync::mpsc::Sender};
use crate::base::QHandler;
use reqwest::Client;
#[derive(Clone)]
pub struct HttpsUpstream {
    url: String,
    client: Client,
}

impl HttpsUpstream { 
    pub async fn build(url: String) -> Result<Box<dyn QHandler>> {
        let client = reqwest::Client::builder()
            .tcp_keepalive(Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()?;
        
        Ok(Box::new(Self {
            url,
            client
        }))
    }
}

#[async_trait]
impl QHandler for HttpsUpstream {
    async fn query(&self, qmsg: Bytes, sender: Sender<Bytes>) -> Result<()> {
        let bytes = self.client.post(self.url.clone())
            .header("content-type", "application/dns-message")
            .header("Connection", "keep-alive")
            .body(qmsg)
            .send()
            .await?.
            bytes()
            .await?;
        tokio::select! {
            _ = sender.closed() => {}
            _ = sender.send_timeout(bytes, Duration::from_millis(500)) => {}
        }
        Ok(())
    }
}
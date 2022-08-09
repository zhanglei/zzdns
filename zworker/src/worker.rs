use anyhow::Result;
use tokio::signal;
use zresolver::ZResolver;
use std::sync::Arc;
use zqueue::{ZRequestQueue, ZResponseQueue, ZQueueHander};



#[derive(Clone)]
pub struct ZWorker {
    name: String,
    req_q: Arc<ZRequestQueue>,
    res_q: Arc<ZResponseQueue>,
    zresolver: Arc<ZResolver>,
}

impl ZWorker {

    pub fn new(id: usize, req_q: Arc<ZRequestQueue>, res_q: Arc<ZResponseQueue>, zresolver: Arc<ZResolver>) -> Self {
        let name = format!("zworker<{}>", id);
        Self { name, req_q, res_q, zresolver }
    }

    pub async fn serve(&self) -> Result<()> {
        info!("{} is running...", self.name);
        loop {
            tokio::select! {
                res = self.req_q.recv() => {
                    let (src, msg) = match res {
                        Ok(r) => r,
                        Err(e) => {
                            error!("{}, Unable to read data, error:{}", self.name, e);
                            continue;
                        }
                    };
                    let msg = match self.zresolver.resolve(msg).await {
                        Ok(r) => r,
                        Err(e) => {
                            error!("{}, Failed to process request, error:{}", self.name, e);
                            continue;
                        }
                    };
                    match self.res_q.send((src, msg)).await {
                        Ok(_) => {}, // info!("{}, successfully processed a request.", self.name);
                        Err(e) => error!("{}, Failed to send message to reply queue, error:{}", self.name, e),
                    }
                }
                _ = signal::ctrl_c() => {
                    return self.stop().await;
                }
            }
        }
    }

    pub async fn start(&self) -> Result<()> { 
        self.serve().await
    }

    pub async fn stop(&self) -> Result<()> {
        Ok(())
    }
}


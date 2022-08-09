use std::sync::Arc;

use async_trait::async_trait;
use anyhow::Result;
use zqueue::{ZRequestQueue, ZResponseQueue};
use super::ServerConf;

pub enum ZServerType {
    Udp = 1,
    Tcp = 2,
    Dot = 3,
    Doh = 4
}


#[async_trait]
pub trait ZServer: Send + Sync + 'static {
    async fn build(req_q: Arc<ZRequestQueue>, res_q: Arc<ZResponseQueue>, conf: ServerConf) -> Result<Self> where Self:Sized;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
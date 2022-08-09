mod base;
mod udp;

#[macro_use] extern crate log;

use anyhow::Result;
use std::sync::Arc;
use udp::UdpZserver;

use zqueue::{ZRequestQueue, ZResponseQueue};
pub use base::{ZServer, ZServerType};
use zconfig::Server as ServerConf;
pub struct ZServerBuilder {}

impl ZServerBuilder {
    
    pub async fn build(conf: ServerConf, req_q: Arc<ZRequestQueue>, res_q: Arc<ZResponseQueue>) -> Result<impl ZServer> {
        UdpZserver::build(req_q, res_q, conf).await
    }
}
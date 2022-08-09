use std::{sync::Arc};
use zcacher::ZCacher;
use zresolver::ZResolver;
use zserver::*;
use zqueue::*;
use zworker::*;
use zconfig::CONFIG;
use zupstream::ZUpstream;
use zpreloader::ZPreloader;

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();
    let qsize = CONFIG.server.qsize.into();
    let worker = CONFIG.server.worker.into();
    let req_q = Arc::new(ZRequestQueue::new(qsize));
    let res_q = Arc::new(ZResponseQueue::new(qsize));
    let zserver = ZServerBuilder::build(CONFIG.server.clone(), req_q.clone(), res_q.clone()).await.unwrap(); 
    let zupstream = Arc::new(ZUpstream::build(CONFIG.upstreams.clone()).await.unwrap());
    let zcacher = Arc::new(ZCacher::new(CONFIG.cache.clone(), zupstream.clone()));
    let zresolver = Arc::new(ZResolver::new(zupstream.clone(), zcacher.clone()));
    let zcacher2 = zcacher.clone();
    for i in 0..worker {
        let zworker = ZWorker::new(i, req_q.clone(), res_q.clone(), zresolver.clone());
        tokio::spawn(async move {
            zworker.start().await.unwrap();
        });
    }
    
    tokio::spawn(async move {
        zcacher.start().await.unwrap();
    });

    tokio::spawn(async move {
        ZPreloader::load(CONFIG.cache.preload_file.clone(), zcacher2.clone()).await;
    });

    zserver.start().await.unwrap();

}

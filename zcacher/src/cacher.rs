
use std::{sync::Arc, collections::HashSet, str::FromStr, time::Duration};
use anyhow::Result;
use async_channel::{bounded, Sender, Receiver};
use bytes::{Bytes, BytesMut};
use domain::base::{MessageBuilder, Question, Rtype, Message, iana::{Rcode, Class}, Dname};
use domain::rdata;
use stretto::AsyncCache;
use tokio::signal;
use zupstream::ZUpstream;
use zconfig::Cache as CacheConf;
use zspeedtest::ZSpeedTest;


#[derive(Clone)]
pub struct ZCacher {
    cache: AsyncCache<String, Bytes>,
    q_sender: Arc<Sender<String>>,
    q_receiver: Arc<Receiver<String>>,
    upstream: Arc<ZUpstream>,
    conf: Arc<CacheConf>,
}

impl ZCacher {
    pub fn new(conf: CacheConf, zupstream: Arc<ZUpstream>) -> Self {
        let cache: AsyncCache<String, Bytes> = AsyncCache::new(conf.max_size.into(), 1e6 as i64,
        tokio::spawn).unwrap();
        let (s, r) = bounded::<String>(conf.max_size.into());
        Self { cache, q_sender: Arc::new(s), q_receiver: Arc::new(r), upstream: zupstream, conf: Arc::new(conf) }
    }
    
    // 接收缓存队列域名解析
    pub async fn serve(&self) -> Result<()> {
        info!("zcacher running...");
        loop {
            tokio::select! {
                res = self.q_receiver.recv() => {
                    let domain = match res {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Unable to read data, error:{}", e);
                            continue;
                        }
                    };
                    let cache = self.cache.clone();
                    let upstream = self.upstream.clone();
                    let conf = self.conf.clone();
                    tokio::spawn(async move {
                        let _ = handle(cache, upstream, conf, domain).await;
                    });

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

    pub async fn get(&self, domain: String) -> Option<(Bytes, u32)> {
        if let Some(res) =  self.cache.get(&domain) {
            let bytes = res.value().clone();
            let ttl = res.ttl().as_secs() as u32;
            let keep_ttl = 15;
            if ttl < keep_ttl {
                let _ = self.push(domain).await;
                Some((bytes, keep_ttl))
            }else{
                Some((bytes, ttl - keep_ttl))
            }
        }else {
            None
        }
    }


    pub async fn push(&self, domain: String) -> Result<()> {
        let _ = self.q_sender.send(domain).await;
        Ok(())
    }
}


async fn handle(cache: AsyncCache<String, Bytes>, upstream: Arc<ZUpstream>, conf: Arc<CacheConf>, domain: String) -> Result<()> {

    if cache.clone().get(&domain).is_some() {
        return Ok(());
    };

    let qname = domain::base::Dname::bytes_from_str(domain.as_str()).unwrap();
    let qmsg_builder = MessageBuilder::from_target(BytesMut::with_capacity(1024))?;
    let mut question_builder = qmsg_builder.question();
    question_builder.push(Question::new_in(qname.clone(), Rtype::A)).unwrap();
    let qmsg = question_builder.into_message().into_octets();

    let rbytes_list = upstream.query_all(&qmsg).await.unwrap();

    let mut cname_list = HashSet::new();
    let mut ip_list = HashSet::new();
    let mut ttl = u32::MAX;

    for rbytes in rbytes_list {
        if let Ok(rmsg) = Message::from_octets(rbytes) {

            let (_, answer, _, _) = match rmsg.sections() {
                Ok(s) => s,
                Err(_) => {continue}
            };

            for rr in answer.flatten() {
                if rr.rtype()  !=  domain::base::Rtype::A && rr.rtype() != domain::base::Rtype::Cname {
                    continue;
                }
                
                if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::A>() {
                    if record.is_some() {
                        let record = record.unwrap();
                        if record.ttl() < ttl {
                            ttl = record.ttl();
                        }
                        ip_list.insert(record.data().addr());
                    }
                }

                if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::Cname<_>>() {
                    if record.is_some() {
                        let record = record.unwrap();
                        if record.ttl() < ttl {
                            ttl = record.ttl();
                        }
                        cname_list.insert(record.data().to_string());  
                    }
                      
                }
            } 
        }
        
    }

    if ttl < conf.min_ttl.into() {
        ttl = conf.min_ttl.into();
    }

    if ttl > conf.max_ttl.into() {
        ttl = conf.max_ttl.into();
    }

    let mut rmsg = MessageBuilder::from_target(BytesMut::with_capacity(1024))?
    .start_answer(&Message::from_octets(qmsg)?, Rcode::NoError)?;
    let header = rmsg.header_mut();
    header.set_ra(true);

    let mut prev_domain = domain;
    let mut cur_domain = prev_domain.clone();
 
    for name in cname_list.iter() {
        cur_domain = name.clone();
        let dname = Dname::bytes_from_str(&prev_domain).unwrap();
        let cname = rdata::Cname::from(Dname::bytes_from_str(name).unwrap());
        rmsg.push((dname, Class::In, ttl, cname)).unwrap();
        prev_domain = name.clone();
    }
    
    let cur_domain = Dname::bytes_from_str(cur_domain.to_string().as_str()).unwrap();
    let ip = ZSpeedTest::query(ip_list.into_iter().collect()).await?;
    rmsg.push((cur_domain.clone(), Class::In, ttl, rdata::A::from_str(&ip.to_string()).unwrap())).unwrap();
    let status = cache.insert_with_ttl(qname.to_string(), rmsg.into_message().into_octets(), 2, Duration::from_secs(100)).await;
    
    info!("cache domain: {:?}, status={:?}", qname.to_string(), status);
    Ok(())
}
use std::sync::Arc;

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use domain::base::{Message, MessageBuilder, iana::Rcode};
use zcacher::ZCacher;
use zupstream::ZUpstream;

#[derive(Clone)]
pub struct ZResolver {
    zupstream: Arc<ZUpstream>,
    cacher: Arc<ZCacher>,
}


impl  ZResolver {
    pub fn new(zupstream: Arc<ZUpstream>, cacher: Arc<ZCacher>) -> Self {
        Self { zupstream, cacher }
    }

    pub async fn resolve(&self, qmsg: Bytes) -> Result<Bytes> {

        let qmsg = Message::from_octets(qmsg)?;

        Ok(match qmsg.sole_question() {
            Ok(_) => {
                match self.matching(qmsg.clone()).await {
                    Ok(r) => {
                        let question = qmsg.sole_question()?;
                        info!("query success, qname: {:?}, qtype: {:?}", question.qname().to_string(), question.qtype());
                        r
                    },
                    Err(err) => {
                        let question = qmsg.sole_question()?;
                        info!("query error, qname: {:?}, qtype: {:?}, error:{:?}", question.qname().to_string(), question.qtype(), err);
                        MessageBuilder::from_target(BytesMut::with_capacity(1024))?
                                .start_answer(&qmsg, Rcode::ServFail)?
                                .into_message().into_octets()
                    }
                }
            },
            Err(_) => {
                MessageBuilder::from_target(BytesMut::with_capacity(1024))?
            .start_answer(&qmsg, Rcode::ServFail)?
            .into_message().into_octets()
            },
        })
    }

    async fn matching(&self, qmsg: Message<Bytes>) -> Result<Bytes>{

        let question = qmsg.sole_question()?;

        let qtype = question.qtype();

        let res = match qtype {
            domain::base::Rtype::A => self.resolve_a(qmsg).await?,
            _ => self.resolve_other(qmsg).await?,
        };
        Ok(res)
    }

    async fn resolve_other(&self, qmsg: Message<Bytes>) -> Result<Bytes> {
        let res = self.zupstream.query(qmsg.into_octets()).await?;

        Ok(res)
    }

    
    async fn resolve_a(&self, qmsg: Message<Bytes>) -> Result<Bytes> {
        let question = qmsg.sole_question()?;
        let qname = question.qname().to_string();
        let mut rmsg = MessageBuilder::from_target(BytesMut::with_capacity(1024))?
                .start_answer(&qmsg, Rcode::NoError)?;
        let header = rmsg.header_mut();
        header.set_ra(true);

        if let Some((bytes, ttl)) = self.cacher.get(qname.to_string()).await {
            let msg = Message::from_octets(bytes)?;
            let (_, answer, _, _) = msg.sections()?;
            for rr in answer.flatten() {
                if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::A>() {
                    if record.is_some() {
                        let mut record = record.unwrap();
                        record.set_ttl(ttl as u32);
                        rmsg.push(record)?;
                    }
                }
                if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::Cname<_>>() {
                    if record.is_some() {
                        let mut record = record.unwrap();
                        record.set_ttl(ttl as u32);
                        rmsg.push(record)?;
                    } 
                }
            }  
            return Ok(rmsg.into_message().into_octets());
        }

        let up_bytes = self.zupstream.query(qmsg.clone().into_octets()).await?;
        let mut has_a = false;

        let up_msg = Message::from_octets(up_bytes)?;
        let (_, ans,_,_) = up_msg.sections()?;
        let ttl = 10;

        for rr in ans.flatten() {
            if rr.rtype()  !=  domain::base::Rtype::A && rr.rtype() != domain::base::Rtype::Cname {
                continue;
            }
            if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::A>() {
                if record.is_some() {
                    let mut record = record.unwrap();
                    record.set_ttl(ttl);
                    rmsg.push(record).unwrap();
                    has_a = true;
                }
            }
            if let Ok(record) = rr.to_record::<domain::rdata::rfc1035::Cname<_>>() {
                if record.is_none() {
                    continue;
                }
                let mut record = record.unwrap();
                record.set_ttl(ttl);
                rmsg.push(record).unwrap();
                
            }
        }
        if has_a {
            let cacher = self.cacher.clone();
            tokio::spawn(async move {
                let _ = cacher.push(qname).await;
            });
        }

        Ok(rmsg.into_message().into_octets())
    }
}
use tokio::{sync::mpsc::Sender};
use async_trait::async_trait;
use anyhow::Result;
use bytes::Bytes;
use dyn_clone::DynClone;

#[async_trait]
pub trait QHandler: DynClone + Send + Sync  {

    async fn query(&self, qmsg: Bytes, sender: Sender<Bytes>) -> Result<()>;

}

dyn_clone::clone_trait_object!(QHandler);

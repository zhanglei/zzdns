use std::{sync::Arc, env, path::PathBuf};

use tokio::fs;
use zcacher::ZCacher;

pub struct ZPreloader;

impl ZPreloader {

    pub async fn load(domain_filename: String, cacher: Arc<ZCacher>) {
        let dir = match env::current_dir(){
            Ok(path) => path,
            Err(_err) => PathBuf::from("."),
        };
        let filepath = dir.join(domain_filename).into_os_string();
        match fs::read_to_string(filepath).await {
            Ok(domain_lines) => {
                let mut count = 0;
                for line in domain_lines.lines() {
                    if cacher.push(line.to_string()).await.is_ok() {
                        count += 1;
                    }
                }
                info!("Pushed domain name list to cache queue, count:{:?}", count);
            },
            Err(e) => {
                error!("Failed to read domain file, error: {:?}", e);
            },
        }
    }
}
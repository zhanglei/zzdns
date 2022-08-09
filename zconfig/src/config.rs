use std::net::Ipv4Addr;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::fs;
use std::env;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config/config.json";

/// 服务 配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub port: u16,  // 端口
    pub worker: u16, // 工人数量
    pub qsize: u16, // 消息队列大小
    pub stype: Option<String>, // 服务器类型, 默认UDP.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cache {
    pub max_size: u16, // 
    pub max_ttl: u16, //
    pub min_ttl: u16, //
    pub preload_file: String,
}

// 上游服务器配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Upstream {
    pub uptype: Option<String>, 
    pub host: String,
    pub port: Option<u16>
}

impl Upstream {

    pub fn get_type(&self) -> String { 
        if self.uptype.is_some() {
            return self.uptype.clone().unwrap();
        }
        match self.get_host().parse::<SocketAddr>() {
            Ok(_) => "udp".to_string(),
            Err(_) => "https".to_string(),
        }
    }

    pub fn get_host(&self) -> String {
        let host:Option<String>= match self.host.parse::<Ipv4Addr>() {
            Ok(h) => Some(format!("{:?}:{}", h, self.port.unwrap_or(53))), 
            Err(_) => None,
        };

        if let Some(r) = host {
            r
        }else{
            url::Url::parse(self.host.as_str()).unwrap().to_string()
        }
        
    }
    
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub server: Server,
    pub upstreams: Vec<Upstream>,
    pub cache: Cache,
}

/// read configuration from json.
macro_rules! read_config { 
    ($struct: ident) => ({ 
        let current_dir = match env::current_dir(){
            Ok(path) => path,
            Err(_err) => PathBuf::from("."),
        };
        
        let config_path = current_dir.join(CONFIG_FILE).into_os_string();
        
        let config_str = match fs::read_to_string(&config_path) {
            Ok(str) => str,
            Err(_) => {
                let config_path = current_dir.join("../").join(CONFIG_FILE).into_os_string();
                match fs::read_to_string(&config_path) {
                    Ok(str) => str,
                    Err(err) => panic!("Fail to read config file(:{:?}), error:{}", config_path, err),
                }
            },
        };
        match serde_json::from_str::<$struct>(&config_str.as_str()){
            Ok(result) => result,
            Err(err) => panic!("Fail to parse config, error:{}", err),
        }
    })
}


lazy_static! { 
    #[derive(Debug)]
    pub static ref CONFIG:Config = read_config!(Config);
}


#[cfg(test)]
mod test {
    use crate::config::CONFIG;
    #[test]
    fn test_config() {
        for item in CONFIG.upstreams.iter() {
            println!("{:?}, {:?}", item.get_host(), item.get_type());
        }
    }
}
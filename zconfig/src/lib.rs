#[macro_use]extern crate lazy_static;

mod config;

pub use config::CONFIG;
pub use config::Upstream;
pub use config::Server;
pub use config::Cache;
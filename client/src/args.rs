use crate::config::Config;
use std::env;
use std::error::Error;
use std::time::Duration;

pub fn parse_args() -> Result<Config, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut config = Config::default();

    for arg in &args {
        if let Some(val) = arg.strip_prefix("--host=") {
            config.host = val.to_string();
        } else if let Some(val) = arg.strip_prefix("--port=") {
            config.port = val.parse::<u16>()?;
        } else if let Some(val) = arg.strip_prefix("--hash=") {
            config.expected_hash = val.to_string();
        } else if let Some(val) = arg.strip_prefix("--connect-timeout=") {
            config.connect_timeout = Duration::from_secs(val.parse::<u64>()?);
        } else if let Some(val) = arg.strip_prefix("--read-write-timeout=") {
            config.read_write_timeout = Duration::from_secs(val.parse::<u64>()?);
        }
    }

    if config.expected_hash.is_empty() {
        Err("Expected hash (--hash=<HASH>) is required".into())
    } else {
        Ok(config)
    }
}

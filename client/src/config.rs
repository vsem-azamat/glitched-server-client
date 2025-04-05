use std::time::Duration;

pub struct Config {
    pub host: String,
    pub port: u16,
    pub expected_hash: String,
    pub connect_timeout: Duration,
    pub read_write_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            expected_hash: String::new(), // обязательное поле, нет умолчания
            connect_timeout: Duration::from_secs(5),
            read_write_timeout: Duration::from_secs(15),
        }
    }
}

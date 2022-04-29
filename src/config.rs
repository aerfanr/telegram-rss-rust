use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn default_db_host() -> String { String::from("localhost") }
fn default_db_port() -> u16 { 6379 }
fn default_expire_delay() -> i32 { 604800 }

// Bot config structure
#[derive(Deserialize, Debug)]
pub struct Config {
    pub sites: Vec<Site>,
    pub news_interval: u64,
    #[serde(default = "default_db_host")]
    pub db_host: String,
    #[serde(default = "default_db_port")]
    pub db_port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Site {
    pub id: String,
    pub url: String,
    #[serde(default = "default_expire_delay")]
    pub expire_delay: i32,
    pub chats: Vec<teloxide::types::ChatId>,
}

// Read config from file
pub fn get_config() -> Result<Config, Box<dyn Error>> {
    let file = File::open("/opt/rss.yaml")?;
    let reader = BufReader::new(file);
    Ok(serde_yaml::from_reader(reader)?)
}
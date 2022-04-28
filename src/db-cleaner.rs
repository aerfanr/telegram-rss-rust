use std::error::Error;

mod config;
use config::get_config;

fn cleanup() -> Result<(), Box<dyn Error>> {
    // Get the config
    let config = get_config()?;

    log::debug!("Config: {:#?}", config);

    log::debug!("Trying to connect to database...");
    let mut db = redis::Client::open(
        format!("redis://{}:{}", config.db_host, config.db_port)
    )?.get_connection()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Run ZREMRANGEBYSCORE for items with scores from 0 to current time
    let count: i32 = redis::cmd("zremrangebyscore")
        .arg("items")
        .arg("0").arg(now)
        .query(&mut db)?;

    log::info!("Removed {} items.", count);

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    log::info!("Starting database cleanup.");

    match cleanup() {
        Ok(_) => (),
        Err(e) => panic!("{}", e)
    }
}
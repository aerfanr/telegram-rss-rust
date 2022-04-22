use tokio::time::{sleep, Duration};
use teloxide::prelude2::*;
use teloxide::utils::command::BotCommand;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::error::Error;
use redis::RedisError;

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    Help,
    Info
}

// Bot config structure
#[derive(Deserialize, Debug)]
struct Config {
    sites: Vec<Site>,
    news_interval: u64,
}

fn default_expire_delay() -> i32 {
    604800
}

#[derive(Deserialize, Debug, Clone)]
struct Site {
    id: String,
    url: String,
    #[serde(default = "default_expire_delay")]
    expire_delay: i32,
    chats: Vec<teloxide::types::ChatId>,
}

struct News {
    message: String,
    items: Vec<String>,
}

// Read the config and store in CONFIG for global access
thread_local!(static CONFIG: Config = match get_config() {
    Ok(c) => c,
    Err(e) => panic!("{}", e)
});

// Read config from file
fn get_config() -> Result<Config, Box<dyn Error>> {
    let file = File::open("/opt/rss.json")?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

// Check if an item exists in database
fn check_item(title: String, db: &mut redis::Connection) -> bool {
    // Query for the score of item.
    // If it was 'nil' return true, else return false
    let score: Result<Option<u64>, RedisError> = redis::cmd("ZSCORE")
        .arg("items")
        .arg(title)
        .query(db);
    match score {
        Err(e) => {
            log::error!("{}", e);
            false // Do not send item if there is an error
        },
        Ok(s) => {
            match s {
                Some(_) => false,
                None => true
            }
        }
    }
}

// Add a list of items to database
fn db_add_items(items: Vec<String>, expire_delay: i32) 
-> Result<(), Box<dyn Error + Send + Sync>> {
    log::debug!("Trying to connect to database...");
    let mut db = redis::Client::open("redis://127.0.0.1/")?
    .get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    for item in items {
        let expire_time;
        if expire_delay < 0 {
            expire_time = u64::MAX;
        } else {
            expire_time = now + expire_delay as u64;
        }
        // Add item title to sorted set 'items' with expire_time as score
        redis::cmd("ZADD")
            .arg("items")
            .arg(expire_time)
            .arg(item)
            .query(&mut db)?;
    }
    log::debug!("Added items to database.");
    Ok(())
}

// Try sending get request, retry if failed
async fn try_get(url: &str)
    -> reqwest::Result<reqwest::Response> {
    let max_tries = 4; //TODO: implement a config option for this
    for _i in 0..max_tries {
        match reqwest::get(url).await {
            Ok(r) => return Ok(r),
            Err(_e) => ()
        }
    }
    reqwest::get(url).await
}

// Get news and generate message text and news title list
async fn get_news(url: &str) -> Result<News, Box<dyn Error + Send + Sync>> {
    log::debug!("Trying to connect to database...");
    let mut db = redis::Client::open("redis://127.0.0.1/")?.get_connection()?;

    let mut message = String::new();
    let mut result_items: Vec<String> = Vec::new();
    let mut length = 0;

    log::debug!("Getting news from {}", url);
    let res = try_get(url).await?.bytes().await?;
    let channel = rss::Channel::read_from(&res[..])?;
    log::debug!("Recieved {} items.", channel.items.len());

    for item in channel.items {
        match item.title {
            None => log::warn!("Found an item with empty title"),
            Some(title) => {
                let item_text = format!("<a href=\"{}\">{}</a>\n\n",
                    item.link.or(Some(String::new())).unwrap(),
                    title
                );
                let item_length = item_text.chars().count();
                // Do not send more than 4096 chars. Telegram has a message
                // length limit.
                // Continue to check if there is another item that fits
                if length + item_length > 4096 { continue; }
                if check_item(title.clone(), &mut db) {
                    log::debug!("New item: {}", title);
                    length += item_length;
                    message.push_str(&item_text);
                    result_items.push(title);
                } else {
                    log::trace!("Old item: {}", title);
                }
            }
        }
    }

    Ok(News {
        message: message,
        items: result_items
    })
}

//handle received commands
async fn answer(bot: AutoSend<Bot>, message: Message, command: Command)
    -> Result<(), Box<dyn Error + Send + Sync>> {
        match command {
            Command::Help => {
                // Reply with command descriptions
                bot.send_message(message.chat.id, Command::descriptions())
                    .reply_to_message_id(message.id)
                    .await?;
            }
            Command::Info => {
                bot.send_message(message.chat.id, format!("Rust telegram RSS bot
                    \nVersion: Commit {}", env!("GIT_HASH")))
                    .reply_to_message_id(message.id)
                    .await?;
            }
        };
        Ok(())
}

// Send news to chat list
async fn send_news(bot: &AutoSend<Bot>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let sites = CONFIG.with(|config| config.sites.clone());
    for site in sites {
        let news = get_news(&site.url).await?;
        // Do not try to send if news is empty
        if news.items.len() == 0 {
            continue;
        }
        for chat in site.chats {
            // TODO: send the message to first chat, then forward to others to
            // prevent sending large payloads
            bot.send_message(chat, &news.message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        db_add_items(news.items, site.expire_delay)?;
    }
    Ok(())
}

// Send updates automatically
async fn news_loop() {
    let bot = Bot::from_env().auto_send();
    let interval = CONFIG.with(|config| config.news_interval);
    loop {
        match send_news(&bot).await {
            Err(e) => log::error!("{}", e),
            Ok(r) => r
        }
        sleep(Duration::from_secs(interval)).await;
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting... version: Commit {}", env!("GIT_HASH"));

    // log bot config
    CONFIG.with(|config| {
        log::debug!("Config: {:#?}", config)
    });

    let bot = Bot::from_env().auto_send();
    log::debug!("Starting dispatcher...");
    let repl = teloxide::repls2::commands_repl(bot, answer, Command::ty());

    log::debug!("Starting news loop...");
    tokio::join!(repl, news_loop());
}

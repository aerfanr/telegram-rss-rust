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
    News
}

// Bot config structure
#[derive(Deserialize, Debug)]
struct Config {
    sites: Vec<Site>,
    chats: Vec<teloxide::types::ChatId>,
    news_interval: u64
}

#[derive(Deserialize, Debug, Clone)]
struct Site {
    id: String,
    url: String
}

struct News {
    message: String,
    titles: Vec<String>
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
    // Query if this title exists in set 'items'
    let score: Result<bool, RedisError> = redis::cmd("SISMEMBER")
        .arg("items")
        .arg(title)
        .query(db);
    match score {
        Err(e) => {
            log::error!("{}", e);
            false // Do not send item if there is an error
        },
        Ok(s) => !s
    }
}

// Add a list of titles to database
fn db_add_items(titles: Vec<String>)
    -> Result<(), Box<dyn Error + Send + Sync>> {
    log::debug!("Trying to connect to database...");
    let mut db = redis::Client::open("redis://127.0.0.1/")?
        .get_connection()?;
    for title in titles {
        // Add item title to set 'items'
        redis::cmd("SADD").arg("items").arg(title)
            .query(& mut db)?;
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
async fn get_news() -> Result<News, Box<dyn Error + Send + Sync>> {
    let sites = CONFIG.with(|config| config.sites.clone());
    log::debug!("Trying to connect to database...");
    let mut db = redis::Client::open("redis://127.0.0.1/")?.get_connection()?;

    let mut message = String::new();
    let mut result_items: Vec<String> = Vec::new();
    let mut length = 0;
    for site in sites {
        log::debug!("Getting news from {}", site.url);
        let res = try_get(&site.url)
            .await?
            .bytes()
            .await?;
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
                        result_items.push(title.clone());
                    } else {
                        log::trace!("Old item: {}", title);
                    }
                }
            }
        }
    }

    Ok(News {
        message: message,
        titles: result_items
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
            Command::News => {
                let news = get_news().await?;
                // Reply with news
                bot.send_message(message.chat.id, news.message)
                    .reply_to_message_id(message.id)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                // Add titles to database; This should not happen in case of a
                // telegram error.
                db_add_items(news.titles)?
            }
        };
        Ok(())
}

// Send news to chat list
async fn send_news(bot: &AutoSend<Bot>)
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let news = get_news().await?;
        let chats = CONFIG.with(|config| config.chats.clone());
        for chat in chats {
            // TODO: send the message to first chat, then forward to others to
            // prevent sending large payloads
            bot.send_message(chat, &news.message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        db_add_items(news.titles)?;
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
    log::info!("Starting...");

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

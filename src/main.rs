use teloxide::prelude2::*;
use teloxide::utils::command::BotCommand;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::error::Error;

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    Help,
    News
}

// Bot config structure
#[derive(Deserialize, Debug)]
struct Config {
    sites: Vec<Site>
}

#[derive(Deserialize, Debug, Clone)]
struct Site {
    id: String,
    url: String
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

// Generate a news message
async fn get_news() -> Result<String, Box<dyn Error + Send + Sync>> {
    let sites = CONFIG.with(|config| config.sites.clone());

    let mut message = String::new();
    for site in sites {
        let res = reqwest::get(site.url)
            .await?
            .bytes()
            .await?;
        let channel = rss::Channel::read_from(&res[..])?;

        let mut i = 0;
        for item in channel.items {
            if i >= 20 { break; } // Do not send more than 20 items
                                  // Telegram has a message length limit
                                  // TODO: make this more accurate
            i += 1;
            match item.title {
                None => (),
                Some(title) => {
                    message.push_str(&format!("<a href=\"{}\">{}</a>\n\n",
                            item.link.or(Some(String::new())).unwrap(),
                            title
                    ))
                }
            }
        }
    }

    Ok(message)
}

//handle received commands
async fn answer(bot: AutoSend<Bot>, message: Message, command: Command)
    -> Result<(), Box<dyn Error + Send + Sync>> {
        match command {
            Command::Help => {
                // Reply with command descriptions
                bot.send_message(message.chat.id, Command::descriptions())
                    .reply_to_message_id(message.id)
                    .await?
            }
            Command::News => {
                // Reply with news
                bot.send_message(message.chat.id, get_news().await?)
                    .reply_to_message_id(message.id)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?
            }
        };
        Ok(())
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    log::info!("Starting...");

    // log bot config
    CONFIG.with(|config| {
        log::debug!("Config: {:#?}", config)
    });

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::commands_repl(bot, answer, Command::ty()).await;
}

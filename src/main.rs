use teloxide::prelude2::*;
use teloxide::utils::command::BotCommand;
use std::error::Error;

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    Help,
    News
}

// Generate a news message
fn get_news() -> String {
    // TODO: Implement get_news
    String::from("This is the news.")
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
                bot.send_message(message.chat.id, get_news())
                    .reply_to_message_id(message.id)
                    .await?
            }
        };
        Ok(())
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    log::info!("Starting...");

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::commands_repl(bot, answer, Command::ty()).await;
}

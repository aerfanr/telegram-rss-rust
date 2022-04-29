# telegram-rss-rust
A simple telegram bot for following rss feeds. In Rust.

## Build
1. Have rustc and cargo installed. You can use [rustup](https://rustup.rs) for this.
2. Install `pkg-config` and `libssl`. Package names may differ in various distros.
3. Build the project with cargo:
```
git clone https://github.com/aerfanr/telegram-rss-rust.git
cd telegram-rss-rust
cargo build
```
You can also create an optimized release build by using:
```
cargo build --release
```

## Usage
1. Create the config file at `/opt/rss.yaml`. You can copy `rss.yaml` and change it.
2. Set `TELOXIDE_TOKEN` environment variable to your Telegram bot token. You can get a bot token from the BotFather ([t.me/BotFather](t.me/BotFather))
3. Set up a redis server
4. Run the program
```
./target/release/telegram-rss-rust
```
or
```
./target/debug/telegram-rss-rust
```

## Configuration
There is an example config file at `rss.yaml` you can copy and modify it.
Here are all the config options:
### sites
An array of all rss feeds. Each feed is a yaml object with following options:

* id: An arbitrary name for the feed. This has no purpose at the moment.
* url: Feed url
* expire_delay: Number of seconds to wait before removing each item from the database. Negetive values mean the items are never deleted. Default is `604800` (a weak)
* chats: An array of numeric telegram chat ids to send the recieved feed items to
### news_interval
Number of seconds to wait after getting feed updates.
### db_host
A string containing the database host. Default value is `"localhost"`.
### db_port
A number containing the database port. Defalt value is `6379`.
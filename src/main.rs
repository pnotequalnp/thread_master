use serenity::{
    async_trait,
    model::{
        channel::{ChannelType, Message},
        gateway::Ready,
        id::ChannelId,
    },
    prelude::*,
};

use serde_json::{to_value, Map, Value};
use url::Url;

struct Handler(Vec<ChannelId>, Map<String, Value>);

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let Self(channels, thread_opts) = self;
        if !msg.author.bot && channels.contains(&msg.channel_id) {
            let opts = parse_message_title(&msg).map(|name| {
                eprintln!("{:?}", name);
                let mut opts = thread_opts.clone();
                opts.insert("name".to_string(), to_value(name).unwrap());
                opts
            });
            match ctx
                .http
                .create_public_thread(
                    msg.channel_id.0,
                    msg.id.0,
                    opts.as_ref().unwrap_or(thread_opts),
                )
                .await
            {
                Ok(_thread) => {}
                Err(err) => eprintln!("Error creating thread: {:?}", err),
            };
        };
    }

    async fn ready(&self, _: Context, ready: Ready) {
        eprintln!("Connected as {}", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = {
        if let Some(token) = std::env::args()
            .nth(1)
            .and_then(|fp| std::fs::read_to_string(fp).ok())
        {
            eprintln!("Using token from file");
            token
        } else {
            eprintln!("No valid token file given, reading from environment");
            std::env::var("DISCORD_TOKEN")
                .expect("Expected a token file or a token in the environment: DISCORD_TOKEN")
        }
    };

    let channels = std::env::var("THREAD_CHANNEL_IDS")
        .expect("Expected a list of channel IDs in the environment: THREAD_CHANNEL_IDS")
        .split(",")
        .map(|s| s.trim().parse::<u64>().map(ChannelId::from))
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to parse channel ID list");

    eprintln!(
        "Using channel IDs from environment: {:?}",
        channels.iter().map(ChannelId::as_u64).collect::<Vec<_>>()
    );

    let opts = {
        let mut opts = serenity::builder::CreateThread::default();
        opts.name("discussion")
            .kind(ChannelType::PublicThread)
            .auto_archive_duration(1440);
        serenity::utils::hashmap_to_json_map(opts.0)
    };

    let mut client = Client::builder(&token)
        .event_handler(Handler(channels, opts))
        .await
        .expect("Error creating client");

    if let Err(err) = client.start().await {
        eprintln!("Client error: {:?}", err);
    }
}

fn only<T>(mut xs: impl Iterator<Item = T>) -> Option<T> {
    let x = xs.next();
    if let None = xs.next() {
        x
    } else {
        None
    }
}

fn parse_url_title(s: &str) -> Option<String> {
    let url = Url::parse(s).ok()?;
    match url.host_str()? {
        "github.com" | "gitlab.com" => url.path_segments()?.nth(1).map(str::to_string),
        _ => only(url.path_segments()?.filter(|p| !p.is_empty()))
            .or(url.host_str())
            .map(str::to_string),
    }
}

fn parse_message_title(msg: &Message) -> Option<String> {
    msg.content
        .contains(": ")
        .then(|| &msg.content)
        .and_then(|c| {
            c.lines()
                .next()
                .and_then(|l| l.split(": ").next())
                .filter(|s| !s.is_empty() && s.len() <= 100)
                .map(str::to_string)
        })
        .or(msg.content.lines().find_map(parse_url_title))
}

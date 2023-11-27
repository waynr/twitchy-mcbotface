use std::thread;

use tracing_subscriber::EnvFilter;

mod bevy_ui;
use bevy_ui::Botface;
use bevy_ui::ChatboxDispatcher;
use bevy_ui::ChatboxState;
mod error;
use error::Result;

pub(crate) use router::pb::{twitch_client::TwitchClient, IrcMessage, StreamIrcRequest};

fn main() -> Result<()> {
    let filter = EnvFilter::from_default_env()
        .add_directive("off".parse()?)
        .add_directive("face=debug".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let bf = Botface::new()?;
    let chatbox_state = bf.chatbox_state();

    thread::spawn(move || {
        if let Err(err) = run_chatbox_dispatcher(chatbox_state) {
            tracing::error!("all (or some) of the async things failed: {err}");
        }
    });

    bf.run();
    Ok(())
}

#[tokio::main]
pub async fn run_chatbox_dispatcher(chatbox_state: ChatboxState) -> Result<()> {
    let mut chatbox_dispatcher = ChatboxDispatcher::new(chatbox_state);
    loop {
        tracing::info!("connecting to Twitch API router gRPC server");
        let mut client = match TwitchClient::connect("http://localhost:50551").await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("connecting to Twitch gRPC router: {e}");
                continue;
            }
        };
        tracing::info!("retrieving IrcMessage stream from router");
        let stream = match client.stream_irc_messages(StreamIrcRequest {}).await {
            Ok(s) => s.into_inner(),
            Err(e) => {
                tracing::warn!("calling StreamIrcMessages: {e}");
                continue;
            }
        };

        match chatbox_dispatcher.run(stream).await {
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::warn!("disconnected from gRPC server: {e}");
                continue;
            }
        }
    }
}

use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use bevy::prelude::*;
use tokio_stream::StreamExt;
use tonic::Streaming;

use super::super::error::{Error, Result};
use super::super::IrcMessage;

#[derive(Default)]
pub(crate) struct ChatMessage {
    pub(crate) user: String,
    pub(crate) message: String,
}

impl From<IrcMessage> for ChatMessage {
    fn from(msg: IrcMessage) -> Self {
        Self {
            user: msg.user,
            message: msg.message,
        }
    }
}

impl From<&ChatMessage> for String {
    fn from(msg: &ChatMessage) -> String {
        format!("{}: {}", msg.user, msg.message)
    }
}

impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

pub struct ChatboxDispatcher {
    state: ChatboxState,
}

impl ChatboxDispatcher {
    pub fn new(state: ChatboxState) -> Self {
        Self { state }
    }

    pub async fn run(&mut self, mut irc_message_stream: Streaming<IrcMessage>) -> Result<()> {
        tracing::info!("running ChatboxDispatcher loop");
        loop {
            match irc_message_stream.next().await {
                Some(Ok(msg)) => {
                    let mut g = match self.state.incoming.lock() {
                        Ok(g) => g,
                        Err(e) => {
                            tracing::warn!("mutex poison error: {:?}", e);
                            tracing::warn!("recovering from poison error");
                            e.into_inner()
                        }
                    };
                    g.push(msg.into());
                }
                Some(Err(status)) => {
                    tracing::warn!(
                        "irc message stream return error: {}",
                        status.code().description()
                    );
                }
                None => {
                    tracing::warn!("gRPC stream disconnected!");
                    return Err(Error::DisconnectedFromTwitchRouterGRPCServer);
                }
            }
        }
    }
}

#[derive(Clone, Resource, Default)]
pub struct ChatboxState {
    pub(crate) messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub(crate) incoming: Arc<Mutex<Vec<ChatMessage>>>,
}

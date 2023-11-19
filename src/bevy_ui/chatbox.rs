use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use bevy::prelude::*;
use twitch_irc::message::PrivmsgMessage;
use twitch_irc::message::ServerMessage;

use crate::irc::MessageDispatcher;

#[derive(Default)]
pub(crate) struct ChatMessage {
    pub(crate) user: String,
    pub(crate) message: String,
}

impl From<PrivmsgMessage> for ChatMessage {
    fn from(msg: PrivmsgMessage) -> Self {
        Self {
            user: msg.sender.name,
            message: msg.message_text,
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
    message_dispatcher: MessageDispatcher,
    state: ChatboxState,
}

impl ChatboxDispatcher {
    pub fn new(message_dispatcher: MessageDispatcher, state: ChatboxState) -> Self {
        Self {
            message_dispatcher,
            state,
        }
    }

    pub async fn run(&mut self) {
        while let Ok(message) = self.message_dispatcher.receiver.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => match self.state.incoming.lock() {
                    Ok(mut incoming) => {
                        incoming.push(msg.into());
                    }
                    Err(e) => eprintln!("{:?}", e),
                },
                _ => (),
            }
        }
    }
}

#[derive(Clone, Resource, Default)]
pub struct ChatboxState {
    pub(crate) messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub(crate) incoming: Arc<Mutex<Vec<ChatMessage>>>,
}

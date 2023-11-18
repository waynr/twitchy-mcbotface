use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use twitch_irc::message::PrivmsgMessage;
use twitch_irc::message::ServerMessage;

use crate::irc::MessageDispatcher;

struct ChatMessage {
    user: String,
    message: String,
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

pub struct ChatboxState {
    messages: Vec<ChatMessage>,
}

impl ChatboxState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

pub struct Chatbox {
    state: Arc<Mutex<ChatboxState>>,
}

// public fns
impl Chatbox {
    pub fn new(state: Arc<Mutex<ChatboxState>>) -> Self {
        Self { state }
    }

    pub fn state(&self) -> Arc<Mutex<ChatboxState>> {
        self.state.clone()
    }
}

pub struct ChatboxDispatcher {
    message_dispatcher: MessageDispatcher,
    state: Arc<Mutex<ChatboxState>>,
}

impl ChatboxDispatcher {
    pub fn new(message_dispatcher: MessageDispatcher, state: Arc<Mutex<ChatboxState>>) -> Self {
        Self {
            message_dispatcher,
            state,
        }
    }

    pub async fn run(&mut self) {
        while let Ok(message) = self.message_dispatcher.receiver.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => match self.state.lock() {
                    Ok(mut cbstate) => {
                        (*cbstate).messages.push(msg.into());
                    }
                    Err(e) => eprintln!("{:?}", e),
                },
                _ => (),
            }
        }
    }
}

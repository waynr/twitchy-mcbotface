use twitch_irc::message::ServerMessage;
use unicode_segmentation::UnicodeSegmentation;

use crate::irc::{ChatMessage, ComponentMessage, MessageDispatcher};

pub struct Commander {
    static_commands_file: String,
    dispatcher: MessageDispatcher,
}

impl Commander {
    pub fn new(static_commands_file: String, dispatcher: MessageDispatcher) -> Self {
        Self {
            static_commands_file,
            dispatcher,
        }
    }

    pub async fn run_commander(&mut self) {
        while let Ok(message) = self.dispatcher.receiver.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => {
                    let mut words = msg.message_text.as_str().split_word_bounds();
                    if words.next() != Some("!") {
                        continue;
                    }
                    match words.next() {
                        Some(word) => match word {
                            "project" => self.send_msg(
                                &msg.channel_login,
                                "https://github.com/waynr/twitchy-mcbotface",
                            ),
                            "meow" => self.send_msg(&msg.channel_login, "woof"),
                            "woof" => self.send_msg(&msg.channel_login, "meow"),
                            "so" => words
                                    .filter(|&word| word != " ")
                                    .for_each(|word| {
                                        self.send_msg(&msg.channel_login, &format!("https://twitch.tv/{}", word));
                                    }),
                            _ => continue,
                        },
                        _ => continue,
                    }
                    // !<botccmd> <arguments>
                }
                _ => continue,
            }
        }
    }

    pub fn send_msg(&self, channel: &str, message: &str) {
        match self
            .dispatcher
            .sender
            .send(ComponentMessage::Chat(ChatMessage {
                channel: channel.to_string(),
                message: message.to_string(),
            })) {
            Err(e) => println!("failed to send message to channel: {}", e),
            _ => (),
        }
    }
}
